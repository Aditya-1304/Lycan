//! Scanline timing, background and object sampling, windows, and color composition.

use crate::{
    CYCLES_PER_FRAME, CYCLES_PER_SCANLINE, FRAMEBUFFER_PIXELS, SCREEN_HEIGHT, SCREEN_WIDTH,
};

/// Register snapshot for one text-background scanline. This is rebuilt at each
/// drawing boundary so guest writes need no tile or register cache invalidation.
struct TextBackground {
    scroll_x: usize,
    width_mask: usize,
    map_row: usize,
    tile_y: usize,
    character_base: usize,
    color_256: bool,
    /// One source tile row, valid only during this immutable drawing boundary.
    tile_column: usize,
    colors: [u16; 8],
    opaque: u8,
}

impl TextBackground {
    fn from_registers(io: &[u8], background: usize, line: usize) -> Self {
        let halfword = |offset| u16::from_le_bytes([io[offset], io[offset + 1]]);
        let control = halfword(8 + background * 2);
        let width = if control & (1 << 14) != 0 { 512 } else { 256 };
        let height = if control & (1 << 15) != 0 { 512 } else { 256 };
        let y = (line + usize::from(halfword(0x12 + background * 4) & 0x1ff)) & (height - 1);
        Self {
            scroll_x: usize::from(halfword(0x10 + background * 4) & 0x1ff),
            width_mask: width - 1,
            map_row: usize::from((control >> 8) & 31) * 0x800
                + (y / 256) * (width / 256) * 0x800
                + (y / 8 % 32) * 64,
            tile_y: y & 7,
            character_base: usize::from((control >> 2) & 3) * 0x4000,
            color_256: control & (1 << 7) != 0,
            tile_column: usize::MAX,
            colors: [0; 8],
            opaque: 0,
        }
    }

    /// Resolves a texel through screen blocks, tile flips, and its palette.
    /// Color index zero is transparent regardless of the selected palette bank.
    fn pixel(&mut self, x: usize, vram: &[u8], palette: &[u8]) -> Option<u16> {
        let x = (x + self.scroll_x) & self.width_mask;
        let column = x / 8;
        if column != self.tile_column {
            self.tile_column = column;
            self.opaque = 0;
            let map_offset = (self.map_row + (x / 256) * 0x800 + (column & 31) * 2) & 0xffff;
            let entry = u16::from_le_bytes([vram[map_offset], vram[map_offset + 1]]);
            let tile_y = if entry & (1 << 11) != 0 {
                7 - self.tile_y
            } else {
                self.tile_y
            };
            let row_bytes = if self.color_256 { 8 } else { 4 };
            let row_offset = self.character_base
                + usize::from(entry & 0x3ff) * row_bytes * 8
                + tile_y * row_bytes;
            // Invalid text character rows remain transparent; OBJ character
            // memory above 64 KiB cannot be sampled as text background data.
            if let Some(row) = vram[..0x10000].get(row_offset..row_offset + row_bytes) {
                let bank = if self.color_256 {
                    0
                } else {
                    usize::from(entry >> 12) * 16
                };
                for local_x in 0..8 {
                    let source_x = if entry & (1 << 10) != 0 {
                        7 - local_x
                    } else {
                        local_x
                    };
                    let color = if self.color_256 {
                        row[source_x]
                    } else {
                        (row[source_x / 2] >> ((source_x & 1) * 4)) & 15
                    };
                    if color != 0 {
                        let offset = (bank + usize::from(color)) * 2;
                        self.colors[local_x] =
                            u16::from_le_bytes([palette[offset], palette[offset + 1]]);
                        self.opaque |= 1 << local_x;
                    }
                }
            }
        }
        let local_x = x & 7;
        (self.opaque & (1 << local_x) != 0).then_some(self.colors[local_x])
    }
}

/// Snapshot of an object and its optional signed 8.8 OAM transformation.
/// Source dimensions determine tile addressing; display bounds determine clipping.
struct Object {
    attr0: u16,
    attr1: u16,
    attr2: u16,
    width: usize,
    height: usize,
    matrix: Option<[i32; 4]>,
}

impl Object {
    fn from_oam(entry: &[u8], oam: &[u8]) -> Option<Self> {
        let attr0 = u16::from_le_bytes([entry[0], entry[1]]);
        let attr1 = u16::from_le_bytes([entry[2], entry[3]]);
        let attr2 = u16::from_le_bytes([entry[4], entry[5]]);
        // Bit 9 disables regular objects but doubles affine display bounds.
        // Object-window texels select masks; semitransparent effects arrive separately.
        let affine = attr0 & 0x100 != 0;
        if attr0 & 0x0c00 == 0x0c00 || (!affine && attr0 & 0x200 != 0) {
            return None;
        }
        let dimensions = match attr0 >> 14 {
            0 => [(8, 8), (16, 16), (32, 32), (64, 64)],
            1 => [(16, 8), (32, 8), (32, 16), (64, 32)],
            2 => [(8, 16), (8, 32), (16, 32), (32, 64)],
            _ => return None,
        };
        let (width, height) = dimensions[usize::from(attr1 >> 14)];
        Some(Self {
            attr0,
            attr1,
            attr2,
            width,
            height,
            matrix: affine.then(|| {
                let base = usize::from((attr1 >> 9) & 31) * 32;
                [6, 14, 22, 30].map(|offset| {
                    i16::from_le_bytes([oam[base + offset], oam[base + offset + 1]]) as i32
                })
            }),
        })
    }

    /// Finishes the final screen-aligned horizontal mosaic block. Both visible
    /// objects and object windows use this coverage; nominal bounds still define
    /// source geometry and vertical clipping. Signed X preserves left-edge wrapping.
    fn horizontal_coverage(&self, io: &[u8]) -> usize {
        let (width, _) = self.bounds();
        if self.attr0 & 0x1000 == 0 {
            return width;
        }
        let mosaic_width = i32::from(io[0x4d] & 15) + 1;
        let raw_x = i32::from(self.attr1 & 511);
        let x = if raw_x >= 256 { raw_x - 512 } else { raw_x };
        let end = x + width as i32;
        width + (mosaic_width - end.rem_euclid(mosaic_width)).rem_euclid(mosaic_width) as usize
    }

    /// Expanded bounds change the center and clipping rectangle, never source stride.
    fn bounds(&self) -> (usize, usize) {
        let factor = if self.matrix.is_some() && self.attr0 & 0x200 != 0 {
            2
        } else {
            1
        };
        (self.width * factor, self.height * factor)
    }

    /// Prepares addressing and vertical transform terms shared by one object row.
    /// The snapshot is also used by object-window coverage and expires at scanout.
    fn row(&self, y: usize, line: usize, control: u16, io: &[u8]) -> ObjectRow<'_> {
        let mosaic = self.attr0 & 0x1000 != 0;
        let mosaic_width = if mosaic {
            usize::from(io[0x4d] & 15) + 1
        } else {
            1
        };
        let y = if mosaic {
            y.saturating_sub(line % (usize::from(io[0x4d] >> 4) + 1))
        } else {
            y
        };
        let (bound_width, bound_height) = self.bounds();
        let color_256 = self.attr0 & 0x2000 != 0;
        let units = if color_256 { 2 } else { 1 };
        let one_dimensional = control & 0x40 != 0;
        let mut base = usize::from(self.attr2 & 0x3ff);
        if color_256 && !one_dimensional {
            base &= !1;
        }
        let transform = self.matrix.map(|[pa, pb, pc, pd]| {
            let dy = y as i32 - bound_height as i32 / 2;
            [
                pa,
                pb * dy - pa * (bound_width as i32 / 2),
                pc,
                pd * dy - pc * (bound_width as i32 / 2),
            ]
        });
        let source_y = if transform.is_none() && self.attr1 & 0x2000 != 0 {
            self.height - 1 - y
        } else {
            y
        };
        ObjectRow {
            object: self,
            mosaic_width,
            transform,
            source_y,
            color_256,
            units,
            one_dimensional,
            base,
            row_stride: if one_dimensional {
                self.width / 8 * units
            } else {
                32
            },
            palette_bank: if color_256 {
                0
            } else {
                usize::from(self.attr2 >> 12) * 16
            },
        }
    }
}

/// Immutable per-row OBJ state; no cached data survives guest VRAM/OAM writes.
struct ObjectRow<'a> {
    object: &'a Object,
    mosaic_width: usize,
    transform: Option<[i32; 4]>,
    source_y: usize,
    color_256: bool,
    units: usize,
    one_dimensional: bool,
    base: usize,
    row_stride: usize,
    palette_bank: usize,
}

impl ObjectRow<'_> {
    /// Applies screen-aligned mosaic before flips/transforms, then samples OBJ RAM.
    /// Character names retain 1D low bits and 2D horizontal wrapping semantics.
    fn pixel(&self, local_x: usize, screen_x: usize, vram: &[u8], palette: &[u8]) -> Option<u16> {
        let mut x = if self.mosaic_width > 1 {
            local_x.saturating_sub(screen_x % self.mosaic_width)
        } else {
            local_x
        };
        let mut y = self.source_y;
        if let Some([pa, bias_x, pc, bias_y]) = self.transform {
            let source_x = ((pa * x as i32 + bias_x) >> 8) + self.object.width as i32 / 2;
            let source_y = ((pc * x as i32 + bias_y) >> 8) + self.object.height as i32 / 2;
            if source_x < 0
                || source_y < 0
                || source_x >= self.object.width as i32
                || source_y >= self.object.height as i32
            {
                return None;
            }
            x = source_x as usize;
            y = source_y as usize;
        } else if self.object.attr1 & 0x1000 != 0 {
            x = self.object.width - 1 - x;
        }
        let tile_x = x / 8;
        let row = (y / 8) * self.row_stride;
        let tile = if self.one_dimensional {
            (self.base + row + tile_x * self.units) & 0x3ff
        } else {
            ((self.base & !31) + row + (((self.base & 31) + tile_x * self.units) & 31)) & 0x3ff
        };
        let offset = 0x10000 + tile * 32;
        let color = if self.color_256 {
            usize::from(vram[offset + (y & 7) * 8 + (x & 7)])
        } else {
            let packed = vram[offset + (y & 7) * 4 + (x & 7) / 2];
            usize::from((packed >> ((x & 1) * 4)) & 15)
        };
        if color == 0 {
            return None;
        }
        let offset = 0x200 + (self.palette_bank + color) * 2;
        Some(u16::from_le_bytes([palette[offset], palette[offset + 1]]))
    }
}

/// Affine source sampling uses signed 8.8 coefficients and signed 28-bit origins.
/// The display owns accumulated origins; this snapshot resolves one scanline.
struct AffineBackground {
    origin: [i32; 2],
    step: [i32; 2],
    width: i32,
    height: i32,
    wrap: bool,
    mode: u16,
    page: usize,
    map: usize,
    character_base: usize,
}

impl AffineBackground {
    /// Captures horizontal coefficients while retaining the display's current line origin.
    fn new(io: &[u8], background: usize, origin: [i32; 2], mode: u16, page: usize) -> Self {
        let base = 0x20 + (background - 2) * 16;
        let control = u16::from_le_bytes([io[8 + background * 2], io[9 + background * 2]]);
        let (width, height) = if mode == 5 {
            (160, 128)
        } else if mode >= 3 {
            (240, 160)
        } else {
            let size = 128 << (control >> 14);
            (size, size)
        };
        Self {
            origin,
            step: [0, 4].map(|offset| {
                i16::from_le_bytes([io[base + offset], io[base + offset + 1]]) as i32
            }),
            width,
            height,
            wrap: mode < 3 && control & 0x2000 != 0,
            mode,
            page,
            map: usize::from((control >> 8) & 31) * 0x800,
            character_base: usize::from((control >> 2) & 3) * 0x4000,
        }
    }

    /// Bitmap bounds always clip; tiled backgrounds may wrap through BGxCNT.
    fn pixel(&self, position: [i32; 2], vram: &[u8], palette: &[u8]) -> Option<u16> {
        let [mut x, mut y] = position.map(|coordinate| coordinate >> 8);
        if self.wrap {
            // Tiled affine dimensions are powers of two. Masking signed source
            // coordinates preserves Euclidean wrapping, including negatives.
            x &= self.width - 1;
            y &= self.height - 1;
        } else if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return None;
        }
        let (x, y) = (x as usize, y as usize);
        if self.mode == 3 || self.mode == 5 {
            let offset =
                (if self.mode == 5 { self.page } else { 0 }) + (y * self.width as usize + x) * 2;
            return Some(u16::from_le_bytes([vram[offset], vram[offset + 1]]));
        }
        let color = if self.mode == 4 {
            vram[self.page + y * 240 + x]
        } else {
            let tile =
                usize::from(vram[(self.map + y / 8 * (self.width as usize / 8) + x / 8) & 0xffff]);
            vram[(self.character_base + tile * 64 + y % 8 * 8 + x % 8) & 0xffff]
        };
        if color == 0 {
            return None;
        }
        let offset = usize::from(color) * 2;
        Some(u16::from_le_bytes([palette[offset], palette[offset + 1]]))
    }
}

/// Window selection precedes layer priority. Bits 0..4 enable BG0..3/OBJ;
/// bit 5 preserves permission for the subsequent color-effects stage.
pub(super) struct WindowMasks {
    pub(super) layers: [u8; SCREEN_WIDTH],
}

impl WindowMasks {
    /// Hardware ranges are half-open and wrap when the start exceeds the end.
    fn contains(bounds: u16, coordinate: usize) -> bool {
        let start = usize::from(bounds >> 8);
        let end = usize::from(bounds & 255);
        if start <= end {
            (start..end).contains(&coordinate)
        } else {
            coordinate >= start || coordinate < end
        }
    }

    /// WIN0 overrides WIN1, which overrides OBJWIN, which overrides WINOUT.
    /// Transparent OBJWIN texels leave the previous outside mask intact.
    fn scanline(
        control: u16,
        line: usize,
        io: &[u8],
        oam: &[u8],
        vram: &[u8],
        palette: &[u8],
    ) -> Self {
        let mut layers = [0x3f; SCREEN_WIDTH];
        if control & 0xe000 == 0 {
            return Self { layers };
        }
        layers.fill(io[0x4a] & 0x3f);
        if control & 0x9000 == 0x9000 {
            for entry in oam.as_chunks::<8>().0 {
                let Some(object) = Object::from_oam(entry, oam) else {
                    continue;
                };
                if object.attr0 & 0x0c00 != 0x0800 {
                    continue;
                }
                let y = (line + 256 - usize::from(object.attr0 & 255)) & 255;
                let (_, height) = object.bounds();
                if y >= height {
                    continue;
                }
                let row = object.row(y, line, control, io);
                for local_x in 0..object.horizontal_coverage(io) {
                    let x = (usize::from(object.attr1 & 511) + local_x) & 511;
                    if x >= SCREEN_WIDTH {
                        continue;
                    }

                    if row.pixel(local_x, x, vram, palette).is_some() {
                        layers[x] = io[0x4b] & 0x3f;
                    }
                }
            }
        }
        for window in (0..2).rev() {
            let horizontal_offset = 0x40 + window * 2;
            let vertical_offset = 0x44 + window * 2;

            let horizontal = u16::from_le_bytes([io[horizontal_offset], io[horizontal_offset + 1]]);

            let vertical = u16::from_le_bytes([io[vertical_offset], io[vertical_offset + 1]]);

            if control & (0x2000 << window) == 0 || !Self::contains(vertical, line) {
                continue;
            }

            let x_start = usize::from(horizontal >> 8);
            let x_end = usize::from(horizontal & 0xff);
            let window_mask = io[0x48 + window] & 0x3f;

            for (x, mask) in layers.iter_mut().enumerate() {
                let inside = if x_start <= x_end {
                    x >= x_start && x < x_end
                } else {
                    x >= x_start || x < x_end
                };

                if inside {
                    *mask = window_mask;
                }
            }
        }
        Self { layers }
    }
}

pub(super) const BLEND_LAYER_OBJ: u8 = 4;
const BLEND_LAYER_BACKDROP: u8 = 5;

/// One opaque surface after layer/window filtering but before color effects.
///
/// `color` intentionally retains RGB555 bit 15 until the final mixer because
/// GBA blend hardware exposes that bit as an additional green precision bit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct LayerPixel {
    pub(super) color: u16,
    pub(super) layer: u8,
    pub(super) priority: u8,
    pub(super) semitransparent: bool,
}

impl LayerPixel {
    #[inline]
    fn backdrop(color: u16) -> Self {
        Self {
            color,
            layer: BLEND_LAYER_BACKDROP,
            priority: 0,
            semitransparent: false,
        }
    }

    /// Lower values are visually closer to the viewer.
    ///
    /// At equal BG/OBJ priority the OBJ wins. Among equal-priority BGs,
    /// lower BG number wins. The backdrop is always below ordinary layers.
    #[inline]
    fn order(self) -> u8 {
        match self.layer {
            BLEND_LAYER_OBJ => self.priority * 5,
            0..=3 => self.priority * 5 + self.layer + 1,
            BLEND_LAYER_BACKDROP => 20,
            _ => u8::MAX,
        }
    }
}

/// Only the first two visible surfaces matter to the GBA color-effects stage.
///
/// Keeping exactly two is intentional: alpha blending may not skip a visible
/// non-second-target surface in order to find a deeper eligible target.
#[derive(Clone, Copy, Debug)]
pub(super) struct PixelStack {
    pub(super) top: LayerPixel,
    second: Option<LayerPixel>,
}

impl PixelStack {
    /// BGs arrive front-to-back. Two opaque BGs make deeper BG sampling
    /// irrelevant, but the backdrop alone never closes this sampling gate.
    #[inline]
    fn backgrounds_complete(&self) -> bool {
        self.second
            .is_some_and(|pixel| pixel.layer != BLEND_LAYER_BACKDROP)
    }

    /// Appends an ordered opaque BG without recomputing per-pixel priority.
    /// The later OBJ pass still uses general insertion against both BGs.
    #[inline]
    fn append_background(&mut self, pixel: LayerPixel) {
        if self.top.layer == BLEND_LAYER_BACKDROP {
            self.second = Some(self.top);
            self.top = pixel;
        } else {
            self.second = Some(pixel);
        }
    }

    #[inline]
    pub(super) fn new(backdrop: u16) -> Self {
        Self {
            top: LayerPixel::backdrop(backdrop),
            second: None,
        }
    }

    #[inline]
    pub(super) fn insert(&mut self, pixel: LayerPixel) {
        let order = pixel.order();
        let top_order = self.top.order();

        if order < top_order {
            self.second = Some(self.top);
            self.top = pixel;
            return;
        }

        if order > top_order && self.second.is_none_or(|second| order < second.order()) {
            self.second = Some(pixel);
        }
    }
}

/// Exact brightness results for the 17 hardware coefficients and channel values.
/// Generated at compile time; six-bit green includes the palette's precision bit.
struct BrightnessTable {
    five: [[u8; 32]; 17],
    six: [[u8; 64]; 17],
}

impl BrightnessTable {
    const fn new(brighten: bool) -> Self {
        let mut table = Self {
            five: [[0; 32]; 17],
            six: [[0; 64]; 17],
        };
        let mut coefficient = 0;
        while coefficient <= 16 {
            let mut value = 0;
            while value < 64 {
                table.six[coefficient][value] = if brighten {
                    (value + (((63 - value) * coefficient + 8) >> 4)) as u8
                } else {
                    (value - ((value * coefficient + 7) >> 4)) as u8
                };
                if value < 32 {
                    table.five[coefficient][value] = if brighten {
                        (value + (((31 - value) * coefficient + 8) >> 4)) as u8
                    } else {
                        (value - ((value * coefficient + 7) >> 4)) as u8
                    };
                }
                value += 1;
            }
            coefficient += 1;
        }
        table
    }

    #[inline]
    fn apply(&self, color: u16, coefficient: u8) -> u16 {
        let coefficient = usize::from(coefficient.min(16));
        let five = &self.five[coefficient];
        let six = &self.six[coefficient];
        let r = u16::from(five[usize::from(color & 31)]);
        let g = u16::from(six[usize::from(((color >> 4) & 62) | (color >> 15))]) >> 1;
        let b = u16::from(five[usize::from((color >> 10) & 31)]);
        r | (g << 5) | (b << 10)
    }
}

static BRIGHTEN: BrightnessTable = BrightnessTable::new(true);
static DARKEN: BrightnessTable = BrightnessTable::new(false);

/// Snapshot of BLDCNT/BLDALPHA/BLDY at one drawing boundary.
#[derive(Clone, Copy, Debug)]
pub(super) struct ColorEffects {
    pub(super) first: u8,
    pub(super) second: u8,
    pub(super) mode: u8,
    pub(super) eva: u8,
    pub(super) evb: u8,
    pub(super) evy: u8,
}

impl ColorEffects {
    pub(super) fn from_io(io: &[u8]) -> Self {
        let read = |offset| u16::from_le_bytes([io[offset], io[offset + 1]]);

        let control = read(0x50);
        let alpha = read(0x52);
        let y = read(0x54);

        Self {
            first: (control & 0x003f) as u8,
            second: ((control >> 8) & 0x003f) as u8,
            mode: ((control >> 6) & 3) as u8,
            eva: ((alpha & 0x001f).min(16)) as u8,
            evb: (((alpha >> 8) & 0x001f).min(16)) as u8,
            evy: ((y & 0x001f).min(16)) as u8,
        }
    }

    #[inline]
    fn is_first_target(self, layer: u8) -> bool {
        self.first & (1u8 << layer) != 0
    }

    #[inline]
    fn is_second_target(self, layer: u8) -> bool {
        self.second & (1u8 << layer) != 0
    }

    /// Uses the hardware-observed extra green precision bit and rounds to nearest.
    #[inline]
    pub(super) fn alpha_blend(first: u16, second: u16, eva: u8, evb: u8) -> u16 {
        let eva = u32::from(eva.min(16));
        let evb = u32::from(evb.min(16));

        let r1 = u32::from(first & 31);
        let g1 = u32::from(((first >> 4) & 62) | (first >> 15));
        let b1 = u32::from((first >> 10) & 31);

        let r2 = u32::from(second & 31);
        let g2 = u32::from(((second >> 4) & 62) | (second >> 15));
        let b2 = u32::from((second >> 10) & 31);

        let r = ((r1 * eva + r2 * evb + 8) >> 4).min(31);
        let g = ((g1 * eva + g2 * evb + 8) >> 4).min(63) >> 1;
        let b = ((b1 * eva + b2 * evb + 8) >> 4).min(31);

        (r | (g << 5) | (b << 10)) as u16
    }

    #[inline]
    pub(super) fn brighten(color: u16, evy: u8) -> u16 {
        BRIGHTEN.apply(color, evy)
    }

    #[inline]
    pub(super) fn darken(color: u16, evy: u8) -> u16 {
        DARKEN.apply(color, evy)
    }

    #[inline]
    pub(super) fn resolve(self, stack: PixelStack, effects_allowed: bool) -> u16 {
        let top = stack.top;

        /*
         * A semi-transparent OBJ forces alpha mode and is a first target
         * regardless of BLDCNT's OBJ-first bit.
         *
         * This forced alpha decision precedes the ordinary window SFX gate.
         * If the immediately lower surface is an enabled second target, alpha
         * wins even when BLDCNT selected brighten/darken.
         */
        if top.layer == BLEND_LAYER_OBJ
            && top.semitransparent
            && let Some(second) = stack.second
            && self.is_second_target(second.layer)
        {
            return Self::alpha_blend(top.color, second.color, self.eva, self.evb);
        }

        // Ordinary alpha/brightness is controlled by the selected window's
        // special-effects bit and BLDCNT first-target selection.
        if !effects_allowed || !self.is_first_target(top.layer) {
            return top.color & 0x7fff;
        }

        match self.mode {
            // None
            0 => top.color & 0x7fff,

            // Alpha. The immediate lower visible surface must itself be an
            // eligible second target; never search further down the stack.
            1 => {
                if let Some(second) = stack.second
                    && self.is_second_target(second.layer)
                {
                    Self::alpha_blend(top.color, second.color, self.eva, self.evb)
                } else {
                    top.color & 0x7fff
                }
            }

            // Brighten
            2 => Self::brighten(top.color, self.evy),

            // Darken
            3 => Self::darken(top.color, self.evy),

            _ => top.color & 0x7fff,
        }
    }
}

/// Display timing is independent of VRAM writes and frontend presentation.
pub(super) struct Display {
    pub(super) control: u16,
    /// Internal BG2/BG3 origins advance independently of the visible MMIO latches.
    affine_origin: [[i32; 2]; 2],
    pub(super) drawing: Vec<u16>,
    pub(super) completed: Vec<u16>,
    pub(super) generation: u64,
    frame_start: u64,
    pub(super) line: usize,
    pub(super) next_event: u64,
}

impl Display {
    pub(super) fn new() -> Self {
        Self {
            control: 0,
            affine_origin: [[0; 2]; 2],
            drawing: vec![0; FRAMEBUFFER_PIXELS],
            completed: vec![0; FRAMEBUFFER_PIXELS],
            generation: 0,
            frame_start: 0,
            line: 0,
            next_event: 960,
        }
    }

    /// Reloads only the written coordinate, including partial byte/halfword stores.
    pub(super) fn write_reference(&mut self, offset: usize, io: &[u8]) {
        if (0x28..0x30).contains(&offset) || (0x38..0x40).contains(&offset) {
            let background = usize::from(offset >= 0x38);
            let axis = (offset & 7) / 4;
            let base = 0x28 + background * 16 + axis * 4;
            let raw = i32::from_le_bytes([io[base], io[base + 1], io[base + 2], io[base + 3]]);
            self.affine_origin[background][axis] = (raw << 4) >> 4;
        }
    }

    /// Isolates the first opaque OBJ texel in OAM order before combining that
    /// single OBJ surface with backgrounds. A later OBJ cannot participate in
    /// OBJ-to-OBJ blending through the color-effects stage.
    pub(super) fn render_objects(
        &self,
        vram: &[u8],
        palette: &[u8],
        oam: &[u8],
        masks: &WindowMasks,
        io: &[u8],
        stacks: &mut [PixelStack; SCREEN_WIDTH],
    ) {
        let mut objects: [Option<LayerPixel>; SCREEN_WIDTH] = [None; SCREEN_WIDTH];

        for entry in oam.as_chunks::<8>().0 {
            let Some(object) = Object::from_oam(entry, oam) else {
                continue;
            };

            let object_mode = object.attr0 & 0x0c00;

            // Object-window pixels affect WindowMasks but are not visible OBJ pixels.
            if object_mode == 0x0800 {
                continue;
            }

            let y = (self.line + 256 - usize::from(object.attr0 & 255)) & 255;
            let (_, bound_height) = object.bounds();

            if y >= bound_height {
                continue;
            }

            let row = object.row(y, self.line, self.control, io);
            for local_x in 0..object.horizontal_coverage(io) {
                let x = (usize::from(object.attr1 & 511) + local_x) & 511;

                if x >= SCREEN_WIDTH || masks.layers[x] & 0x10 == 0 || objects[x].is_some() {
                    continue;
                }

                if let Some(color) = row.pixel(local_x, x, vram, palette) {
                    objects[x] = Some(LayerPixel {
                        color,
                        layer: BLEND_LAYER_OBJ,
                        priority: ((object.attr2 >> 10) & 3) as u8,
                        semitransparent: object_mode == 0x0400,
                    });
                }
            }
        }

        for (stack, object) in stacks.iter_mut().zip(objects) {
            if let Some(object) = object {
                stack.insert(object);
            }
        }
    }

    /// Renders each visible scanline at its drawing boundary and publishes at VBlank.
    /// Within-line register effects remain the documented scanline approximation.
    pub(super) fn synchronize_to(
        &mut self,
        target: u64,
        vram: &[u8],
        palette: &[u8],
        io: &[u8],
        oam: &[u8],
    ) {
        while self.next_event <= target {
            if self.line < SCREEN_HEIGHT {
                let mode = self.control & 7;

                if self.line == 0 {
                    for offset in [0x28, 0x2c, 0x38, 0x3c] {
                        self.write_reference(offset, io);
                    }
                }

                let page = if self.control & 0x10 != 0 { 0xa000 } else { 0 };

                let forced_blank = self.control & 0x80 != 0;
                let start = self.line * SCREEN_WIDTH;

                if forced_blank {
                    self.drawing[start..start + SCREEN_WIDTH].fill(0x7fff);
                } else {
                    // Preserve the raw palette bit 15 until color effects finish.
                    let backdrop = u16::from_le_bytes([palette[0], palette[1]]);

                    let masks =
                        WindowMasks::scanline(self.control, self.line, io, oam, vram, palette);

                    let mut stacks = [PixelStack::new(backdrop); SCREEN_WIDTH];

                    /*
                     * Produce visible BG surfaces independently of final priority
                     * composition. PixelStack retains exactly the two nearest
                     * nontransparent surfaces required by the effects hardware.
                     *
                     * MOSAIC's BG dimensions are scanline-global register state, so
                     * decode them once rather than once per pixel.
                     */
                    let mosaic_width = usize::from(io[0x4c] & 0x0f) + 1;
                    let mosaic_height = usize::from(io[0x4c] >> 4) + 1;

                    // Immutable drawing-boundary snapshots are decoded once.
                    // Front-to-back BG order permits discarding obscured samples
                    // without changing the two surfaces retained for OBJ/blending.
                    let bg_controls: [u16; 4] = std::array::from_fn(|background| {
                        let offset = 8 + background * 2;
                        u16::from_le_bytes([io[offset], io[offset + 1]])
                    });
                    for priority in 0..4 {
                        for (background, &bg_control) in bg_controls.iter().enumerate() {
                            if self.control & (1 << (8 + background)) == 0
                                || bg_control & 3 != u16::from(priority)
                            {
                                continue;
                            }

                            let text = mode == 0 || (mode == 1 && background < 2);

                            let affine = (mode == 1 && background == 2)
                                || (mode == 2 && background >= 2)
                                || ((3..=5).contains(&mode) && background == 2);

                            if !text && !affine {
                                continue;
                            }

                            let mosaic = bg_control & 0x40 != 0;

                            let sample_y = if mosaic {
                                self.line / mosaic_height * mosaic_height
                            } else {
                                self.line
                            };

                            if text {
                                /*
                                 * TextBackground contains only scanline-invariant register
                                 * state, so construct it once for the complete BG scanline.
                                 */
                                let mut text_layer =
                                    TextBackground::from_registers(io, background, sample_y);
                                let group_width = if mosaic { mosaic_width } else { 1 };
                                let mut sample_x = 0;
                                let mut next_group = group_width;

                                for (x, stack) in stacks.iter_mut().enumerate() {
                                    if x == next_group {
                                        sample_x = x;
                                        next_group += group_width;
                                    }
                                    if stack.backgrounds_complete()
                                        || masks.layers[x] & (1u8 << background) == 0
                                    {
                                        continue;
                                    }

                                    if let Some(color) = text_layer.pixel(sample_x, vram, palette) {
                                        stack.append_background(LayerPixel {
                                            color,
                                            layer: background as u8,
                                            priority,
                                            semitransparent: false,
                                        });
                                    }
                                }
                            } else {
                                /*
                                 * Vertical affine deltas and the scanline's affine origin
                                 * are invariant across X and shared by this row.
                                 */
                                let origin = std::array::from_fn(|axis| {
                                    let offset = 0x22 + (background - 2) * 16 + axis * 4;

                                    let delta =
                                        i16::from_le_bytes([io[offset], io[offset + 1]]) as i32;

                                    /*
                                     * Vertical mosaic samples the first scanline of the
                                     * current mosaic group while the hardware affine
                                     * accumulator itself continues advancing each line.
                                     */
                                    self.affine_origin[background - 2][axis]
                                        .wrapping_sub(delta * (self.line - sample_y) as i32)
                                });

                                /*
                                 * PA/PC horizontal steps, BGCNT and the adjusted line origin
                                 * remain constant across all 240 pixels.
                                 */
                                let affine_layer =
                                    AffineBackground::new(io, background, origin, mode, page);
                                let group_width = if mosaic { mosaic_width } else { 1 };
                                let mut position = affine_layer.origin;
                                let step =
                                    affine_layer.step.map(|value| value * group_width as i32);
                                let mut next_group = group_width;

                                for (x, stack) in stacks.iter_mut().enumerate() {
                                    if x == next_group {
                                        for (coordinate, delta) in position.iter_mut().zip(step) {
                                            *coordinate = coordinate.wrapping_add(delta);
                                        }
                                        next_group += group_width;
                                    }
                                    if stack.backgrounds_complete()
                                        || masks.layers[x] & (1u8 << background) == 0
                                    {
                                        continue;
                                    }

                                    if let Some(color) = affine_layer.pixel(position, vram, palette)
                                    {
                                        stack.append_background(LayerPixel {
                                            color,
                                            layer: background as u8,
                                            priority,
                                            semitransparent: false,
                                        });
                                    }
                                }
                            }
                        }
                    }

                    if self.control & (1 << 12) != 0 {
                        self.render_objects(vram, palette, oam, &masks, io, &mut stacks);
                    }

                    let effects = ColorEffects::from_io(io);

                    for (x, stack) in stacks.into_iter().enumerate() {
                        self.drawing[start + x] =
                            effects.resolve(stack, masks.layers[x] & 0x20 != 0);
                    }
                }

                /*
                 * Affine internal reference points keep advancing even if this
                 * particular scanline was forced blank.
                 */
                for (background, origin) in self.affine_origin.iter_mut().enumerate() {
                    for (axis, coordinate) in origin.iter_mut().enumerate() {
                        let offset = 0x22 + background * 16 + axis * 4;
                        let delta = i16::from_le_bytes([io[offset], io[offset + 1]]) as i32;

                        *coordinate = (coordinate.wrapping_add(delta) << 4) >> 4;
                    }
                }

                self.line += 1;

                self.next_event = self.frame_start
                    + self.line as u64 * CYCLES_PER_SCANLINE
                    + if self.line == SCREEN_HEIGHT { 0 } else { 960 };
            } else {
                std::mem::swap(&mut self.drawing, &mut self.completed);
                self.generation += 1;
                self.frame_start += CYCLES_PER_FRAME;
                self.line = 0;
                self.next_event = self.frame_start + 960;
            }
        }
    }
}

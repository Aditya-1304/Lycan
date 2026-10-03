//! Memory bus, cartridge timing, DMA arbitration, and hardware event scheduling.

use super::InputEvent;
use super::cpu::{Access, AccessKind, CpuBus};
use super::display::Display;
#[cfg(feature = "eeprom-trace")]
use crate::EepromTraceEvent;
use crate::{
    BIOS_SIZE, BIOS_UNDOCUMENTED_IO_410, ButtonState, CYCLES_PER_FRAME, CYCLES_PER_SCANLINE,
    CoreError, EWRAM_START, HBLANK_FLAG_CYCLE, IO_BYTES, IO_START, IWRAM_START, OAM_BYTES,
    OAM_START, PALETTE_BYTES, PALETTE_START, ROM_START, SCANLINES_PER_FRAME, SCREEN_HEIGHT,
    TEST_FIRMWARE, VRAM_BYTES, VRAM_START, audio, eeprom, rtc,
};
use std::{collections::VecDeque, ops::Range};

/// Decoded WAITCNT timing shared by opcode, data and idle bus cycles.
/// Register writes replace this value after completing their own timed access;
/// sequential eligibility and 128 KiB boundaries remain access-local decisions.
struct Waitstates {
    first: [u64; 3],
    sequential: [u64; 3],
    sram: u64,
    prefetch: bool,
}

impl Waitstates {
    fn from_control(control: u16) -> Self {
        Self {
            first: [2, 5, 8].map(|shift| [5, 4, 3, 9][((control >> shift) & 3) as usize]),
            sequential: std::array::from_fn(|bank| {
                if control & (1 << [4, 7, 10][bank]) != 0 {
                    2
                } else {
                    [3, 5, 9][bank]
                }
            }),
            sram: [5, 4, 3, 9][(control & 3) as usize],
            prefetch: control & 0x4000 != 0,
        }
    }

    fn beat_cycles(&self, address: u32, sequential: bool) -> u64 {
        let bank = ((address >> 25) - 4) as usize;
        if sequential && address & 0x1ffff != 0 {
            self.sequential[bank]
        } else {
            self.first[bank]
        }
    }
}

/// Cartridge bus ownership and the eight-halfword opcode FIFO. Addresses stay
/// in their waitstate window; changing windows cannot reuse another bank's fill.
#[derive(Default)]
struct GamePak {
    /// Contiguous address eligible for the next sequential cartridge access.
    next_access: Option<u32>,
    /// Distinguishes opcode fetch continuity from a cartridge data transfer.
    fetch_stream: bool,
    /// Address of the next halfword the CPU can consume from the opcode FIFO.
    head: Option<u32>,
    /// Completed halfwords available to the CPU, bounded by the eight-entry FIFO.
    buffered: u32,
    /// Cycles already spent fetching the next unbuffered halfword.
    progress: u64,
}

impl GamePak {
    /// Fills only while the cartridge bus is free, retaining an unfinished beat.
    fn fill(&mut self, timing: &Waitstates, cycles: u64) {
        if !timing.prefetch {
            return;
        }
        let Some(head) = self.head else {
            return;
        };
        self.progress += cycles;
        while self.buffered < 8 {
            let address = head + self.buffered * 2;
            if !(0x08000000..0x0e000000).contains(&address) {
                self.progress = 0;
                break;
            }
            let cost = timing.beat_cycles(address, true);
            if self.progress < cost {
                break;
            }
            self.progress -= cost;
            self.buffered += 1;
        }
        if self.buffered == 8 {
            self.progress = 0;
        }
    }

    /// Consumes instruction beats without letting data reads enter the FIFO.
    /// A matching FIFO address survives CPU accesses to internal RAM; cartridge
    /// data, redirects, and WAITCNT writes discard the old stream instead.
    fn access(&mut self, timing: &Waitstates, address: u32, width: usize, access: Access) -> u64 {
        let fetch = matches!(access.kind, AccessKind::Fetch);
        let enabled = timing.prefetch;
        let hit = fetch && enabled && self.head == Some(address);
        let sequential =
            access.sequential && self.next_access == Some(address) && self.fetch_stream == fetch;
        if !hit {
            self.head = None;
            self.buffered = 0;
            self.progress = 0;
        }
        let mut cycles = 0;
        for beat in 0..(width / 2).max(1) as u32 {
            let current = address + beat * 2;
            if hit && self.buffered != 0 {
                self.buffered -= 1;
            } else {
                let cost = timing.beat_cycles(current, beat != 0 || sequential || hit);
                cycles += cost.saturating_sub(self.progress);
                self.progress = 0;
            }
        }
        let next = address + (width as u32).max(2);
        self.next_access = Some(next);
        self.fetch_stream = fetch;
        self.head = (fetch && enabled).then_some(next);
        if cycles == 0 {
            // A FIFO hit occupies one CPU cycle while the cartridge keeps filling.
            self.fill(timing, 1);
            1
        } else {
            cycles
        }
    }
}

/// A latched DMA descriptor. Programmed registers remain distinct from these
/// internal pointers so repeat transfers can reload count/destination without
/// rewinding the progressing source.
#[derive(Default)]
pub(super) struct DmaTransfer {
    pub(super) source: u32,
    pub(super) destination: u32,
    initial_destination: u32,
    pub(super) count: u32,
    pub(super) remaining: u32,
    pub(super) control: u16,
    pub(super) active: bool,
    sequential: bool,
}

impl DmaTransfer {
    #[inline]
    fn enabled_for(&self, timing: u16) -> bool {
        self.control & 0x8000 != 0 && (self.control >> 12) & 3 == timing
    }
}

/// Cached executable backing classification, never decoded bytes or a pointer.
/// RAM writes (including DMA) are therefore visible on the next bus fetch;
/// instructions already in the CPU pipeline retain architectural behavior.
#[derive(Clone, Copy)]
enum FetchRegion {
    Ewram,
    Iwram,
    Rom,
    Other,
}

pub(super) struct System {
    pub(super) dma: [DmaTransfer; 4],
    pub(super) audio: audio::Audio,
    /// HALT stops instruction retirement while hardware time continues.
    pub(super) halted: bool,
    pub(super) buttons: ButtonState,
    pub(super) inputs: VecDeque<InputEvent>,
    pub(super) rom: Vec<u8>,
    /// Cartridge clock settings outlive reset; GPIO transfer state does not.
    pub(super) rtc: Option<rtc::Rtc>,
    /// Cartridge-owned SRAM persists through CPU reset; host storage is separate.
    pub(super) sram: Option<crate::sram::Sram>,
    /// Flash commands and bytes share the cartridge lifecycle, not host storage.
    pub(super) flash: Option<crate::flash::Flash>,
    /// Serial cartridge backup occupies the EEPROM window in ROM waitstate two.
    pub(super) eeprom: Option<eeprom::Eeprom>,
    /// Explicitly mapped original test firmware; absent for normal ROM loads.
    pub(super) test_firmware: bool,
    /// User-supplied firmware is owned separately from controlled diagnostics.
    pub(super) bios: Option<Box<[u8; BIOS_SIZE]>>,
    /// Last successful opcode bus fetch in BIOS, including pipeline lookahead.
    pub(super) bios_latch: u32,
    pub(super) bios_enabled: bool,
    pub(super) execution_address: u32,
    pub(super) ewram: Vec<u8>,
    iwram: Vec<u8>,
    pub(super) io: Vec<u8>,
    /// Shared BG/OBJ color RAM; mode 4 indexes the first 256 entries.
    pub(super) palette: Vec<u8>,
    pub(super) vram: Vec<u8>,
    /// 128 eight-byte object entries; the fourth halfword also stores affine data.
    pub(super) oam: Vec<u8>,
    pub(super) display: Display,
    pub(super) cycles: u64,
    /// Device state is materialized at this clock; ordinary RAM/ROM accesses
    /// may advance the bus clock without revisiting devices before an event.
    devices_at: u64,
    /// Audio can be materialized by a register/FIFO write independently of
    /// display, input and timed DMA bookkeeping.
    audio_at: u64,
    /// Earliest observable device/input edge, invalidated by MMIO mutations.
    pub(super) next_device_event: u64,
    /// MMIO mutations force the event-bounded CPU loop to recheck IRQ/DMA/HALT.
    pub(super) cpu_boundary: bool,
    fetch_tag: u32,
    fetch_region: FetchRegion,
    gamepak: GamePak,
    waitstates: Waitstates,
    /// Last driven word supplies otherwise unmapped data reads.
    open_bus: u32,
    /// Instruction prefetch remains independent of the DMA/data bus latch.
    pub(super) cpu_open_bus: u32,
}

impl System {
    pub(super) fn new() -> Self {
        let mut system = Self {
            dma: std::array::from_fn(|_| DmaTransfer::default()),
            audio: audio::Audio::new(),
            halted: false,
            buttons: ButtonState::default(),
            inputs: VecDeque::new(),
            rom: Vec::new(),
            rtc: None,
            sram: None,
            flash: None,
            eeprom: None,
            test_firmware: false,
            bios: None,
            bios_latch: 0,
            bios_enabled: false,
            execution_address: ROM_START,
            ewram: vec![0; 256 * 1024],
            iwram: vec![0; 32 * 1024],
            io: vec![0; IO_BYTES],
            palette: vec![0; PALETTE_BYTES],
            vram: vec![0; VRAM_BYTES],
            oam: vec![0; OAM_BYTES],
            display: Display::new(),
            cycles: 0,
            devices_at: 0,
            audio_at: 0,
            next_device_event: 0,
            cpu_boundary: false,
            fetch_tag: u32::MAX,
            fetch_region: FetchRegion::Other,
            gamepak: GamePak::default(),
            waitstates: Waitstates::from_control(0),
            open_bus: 0,
            cpu_open_bus: 0,
        };
        // Controlled cartridge startup supplies the BIOS-style identity transform.
        // Supplied-BIOS startup clears these values so firmware owns initialization.
        for offset in [0x20, 0x26, 0x30, 0x36] {
            system.io[offset..offset + 2].copy_from_slice(&256i16.to_le_bytes());
        }
        system.refresh_status();
        system
    }

    /// HALT observes enabled pending sources independently of IME and CPSR.I.
    pub(super) fn pending_interrupts(&self) -> u16 {
        u16::from_le_bytes([self.io[0x200], self.io[0x201]])
            & u16::from_le_bytes([self.io[0x202], self.io[0x203]])
    }

    /// KEYCNT evaluates active-low KEYINPUT as pressed selection bits. Requests
    /// latch in IF independently of IE/IME; the shared IRQ path decides delivery.
    pub(super) fn evaluate_keypad(&mut self) {
        let control = u16::from_le_bytes([self.io[0x132], self.io[0x133]]);
        let selected = control & 0x03ff;
        let pressed = self.buttons.0 & selected;
        let matched = if control & 0x8000 != 0 {
            selected != 0 && pressed == selected
        } else {
            pressed != 0
        };
        if control & 0x4000 != 0 && matched {
            self.io[0x203] |= 0x10;
        }
    }

    /// HALT must stop at input, audio, and display-DMA request boundaries rather
    /// than advancing past a timed transfer that affects the next visible line.
    pub(super) fn next_wake_event(&self) -> u64 {
        let mut display = self.next_vblank();

        if self.dma.iter().any(|dma| dma.enabled_for(2)) {
            display = display.min(self.next_visible_hblank());
        }
        if self.io[4] & 0x10 != 0 {
            display = display.min(self.next_hblank());
        }

        if self.io[4] & 0x20 != 0
            && let Some(vcount) = self.next_vcount_match()
        {
            display = display.min(vcount);
        }

        let display = display.min(self.audio.next_event(self.audio_at));

        self.inputs
            .front()
            .map_or(display, |event| display.min(event.cycle.0))
    }

    /// HBlank DMA starts after the visible pixel interval on visible lines.
    fn next_hblank(&self) -> u64 {
        let edge = self.cycles / CYCLES_PER_SCANLINE * CYCLES_PER_SCANLINE + HBLANK_FLAG_CYCLE;
        if edge > self.cycles {
            edge
        } else {
            edge + CYCLES_PER_SCANLINE
        }
    }

    /// Returns the next visible-line HBlank that may issue an ordinary timed
    /// DMA request. HBlank continues during VBlank, but those blank lines must
    /// not consume entries from a repeated raster table.
    pub(super) fn next_visible_hblank(&self) -> u64 {
        let edge = self.next_hblank();
        let line = edge / CYCLES_PER_SCANLINE % SCANLINES_PER_FRAME;

        if line < SCREEN_HEIGHT as u64 {
            edge
        } else {
            (edge / CYCLES_PER_FRAME + 1) * CYCLES_PER_FRAME + HBLANK_FLAG_CYCLE
        }
    }

    /// Returns the next scanline boundary matching DISPSTAT's VCOUNT compare.
    ///
    /// VCOUNT only ranges from 0 through 227. Compare values outside that range
    /// cannot match. An IRQ is generated on the transition into the matching
    /// scanline, not repeatedly while execution remains within that scanline.
    fn next_vcount_match(&self) -> Option<u64> {
        let compare = u64::from(self.io[5]);

        if compare >= SCANLINES_PER_FRAME {
            return None;
        }

        let frame_start = self.cycles / CYCLES_PER_FRAME * CYCLES_PER_FRAME;
        let edge = frame_start + compare * CYCLES_PER_SCANLINE;

        Some(if edge > self.cycles {
            edge
        } else {
            edge + CYCLES_PER_FRAME
        })
    }

    /// A low-to-high enable transition latches the descriptor. DMA0-2 use
    /// 14-bit counts and DMA3 uses 16 bits. DMA0 source and DMA0-2 destination
    /// addresses use 27 bits; the other source/destination ranges use 28 bits.
    fn configure_dma(&mut self, channel: usize, value: u16) {
        let base = 0xb0 + channel * 12;
        let sound = matches!(channel, 1 | 2) && value & 0x3000 == 0x3000;
        let old = self.dma[channel].control;
        self.dma[channel].control = value;
        if value & 0x8000 == 0 {
            self.dma[channel].active = false;
            return;
        }

        if old & 0x8000 != 0 {
            return;
        }

        let width = if sound || value & 0x0400 != 0 { 4 } else { 2 };
        let source_mask = if channel == 0 {
            0x07ff_ffff
        } else {
            0x0fff_ffff
        };
        let destination_mask = if channel == 3 {
            0x0fff_ffff
        } else {
            0x07ff_ffff
        };

        let source = u32::from_le_bytes([
            self.io[base],
            self.io[base + 1],
            self.io[base + 2],
            self.io[base + 3],
        ]);

        let destination = u32::from_le_bytes([
            self.io[base + 4],
            self.io[base + 5],
            self.io[base + 6],
            self.io[base + 7],
        ]);

        self.dma[channel].source = source & source_mask & !(width - 1);

        self.dma[channel].destination = destination & destination_mask & !(width - 1);
        self.dma[channel].initial_destination = self.dma[channel].destination;

        let programmed_count = u16::from_le_bytes([self.io[base + 8], self.io[base + 9]]);
        let latched_count = if channel == 3 {
            u32::from(programmed_count)
        } else {
            u32::from(programmed_count & 0x3fff)
        };

        self.dma[channel].count = if sound {
            // FIFO DMA always transfers four 32-bit units per refill request.
            4
        } else if latched_count == 0 {
            if channel == 3 { 0x1_0000 } else { 0x4000 }
        } else {
            latched_count
        };
        self.dma[channel].remaining = self.dma[channel].count;

        // Immediate transfers own the bus now; timed transfers await a request.
        self.dma[channel].active = value & 0x3000 == 0;
        self.dma[channel].sequential = false;
    }

    /// Services one read/write beat, preserving state between host deadlines.
    /// CPU execution and IRQ entry wait for DMA to release bus ownership. Every
    /// memory access advances scanout and requests through the ordinary bus path.
    pub(super) fn dma_beat(&mut self, channel: usize) -> Result<(), CoreError> {
        let control = self.dma[channel].control;
        let sound = matches!(channel, 1 | 2) && control & 0x3000 == 0x3000;
        let width = if sound || control & 0x400 != 0 { 4 } else { 2 };
        if !self.dma[channel].sequential {
            self.gamepak = GamePak::default();
            self.advance_time(2);
        }
        // Observe both DMA directions before any serial clocks are consumed.
        #[cfg(feature = "eeprom-trace")]
        if channel == 3
            && !self.dma[channel].sequential
            && (self.eeprom_address(self.dma[channel].source)
                || self.eeprom_address(self.dma[channel].destination))
            && let Some(eeprom) = self.eeprom.as_mut()
        {
            eeprom.trace_cycle(self.cycles);
            eeprom.record(EepromTraceEvent::Dma {
                source: self.dma[channel].source,
                destination: self.dma[channel].destination,
                count: self.dma[channel].count,
                width,
            });
        }
        if channel == 3
            && width == 2
            && !self.dma[channel].sequential
            && self.eeprom_address(self.dma[channel].destination)
        {
            let count = self.dma[channel].count;

            if let Some(eeprom) = self.eeprom.as_mut() {
                eeprom.begin_dma(count);
            }
        }
        let access = Access {
            kind: AccessKind::Data,
            sequential: self.dma[channel].sequential,
        };
        if width == 4 {
            let value = self.read32_impl(self.dma[channel].source, access)?;
            self.write32_impl(self.dma[channel].destination, value, access)?;
        } else {
            let value = self.read16_impl(self.dma[channel].source, access)?;
            self.write16_impl(self.dma[channel].destination, value, access)?;
        }
        let advance = |address: u32, mode: u16| match mode {
            1 => address.wrapping_sub(width),
            2 => address,
            _ => address.wrapping_add(width),
        };
        self.dma[channel].source = advance(self.dma[channel].source, (control >> 7) & 3);
        if !sound {
            self.dma[channel].destination =
                advance(self.dma[channel].destination, (control >> 5) & 3);
        }
        self.dma[channel].sequential = true;
        self.dma[channel].remaining -= 1;
        if self.dma[channel].remaining == 0 {
            self.dma[channel].active = false;
            self.gamepak = GamePak::default();
            if control & 0x4000 != 0 {
                self.io[0x203] |= 1 << channel;
            }
            if control & 0x200 != 0 && control & 0x3000 != 0 {
                self.dma[channel].remaining = self.dma[channel].count;
                if control & 0x60 == 0x60 {
                    self.dma[channel].destination = self.dma[channel].initial_destination;
                }
                self.dma[channel].sequential = false;
            } else {
                self.dma[channel].control &= !0x8000;
                self.io[0xb0 + channel * 12 + 11] &= !0x80;
            }
        }
        Ok(())
    }

    /// Absolute next VBlank edge; sleeping callers clip it to their own deadline.
    fn next_vblank(&self) -> u64 {
        let edge = self.cycles / CYCLES_PER_FRAME * CYCLES_PER_FRAME
            + SCREEN_HEIGHT as u64 * CYCLES_PER_SCANLINE;
        if edge > self.cycles {
            edge
        } else {
            edge + CYCLES_PER_FRAME
        }
    }

    /// Refreshes read-only register bits from hardware time, even during blank lines.
    /// Interrupt requests are latched at event crossings, not status reads.
    pub(super) fn refresh_status(&mut self) {
        let status = self.display_status();
        self.io[4..8].copy_from_slice(&status);
        self.audio
            .refresh_timers_at(&mut self.io, self.cycles - self.audio_at);
        self.audio.refresh_controls(&mut self.io);
        self.io[0x130..0x132].copy_from_slice(&(!self.buttons.0 & 0x03ff).to_le_bytes());
    }

    /// DISPSTAT flags are derived from time; reading them never raises IF.
    fn display_status(&self) -> [u8; 4] {
        let line = (self.cycles / CYCLES_PER_SCANLINE % SCANLINES_PER_FRAME) as u16;
        let control = u16::from_le_bytes([self.io[4], self.io[5]]) & 0xff38;
        let flags = u16::from((160..227).contains(&line))
            | (u16::from(self.cycles % CYCLES_PER_SCANLINE >= HBLANK_FLAG_CYCLE) << 1)
            | (u16::from(line == control >> 8) << 2);
        let status = (control | flags).to_le_bytes();
        let count = line.to_le_bytes();
        [status[0], status[1], count[0], count[1]]
    }

    /// Materialize only the register family touched by this MMIO transfer.
    /// VCOUNT polling must not copy timer and sound readbacks on every iteration.
    fn refresh_io_access(&mut self, address: u32, width: usize) {
        let start = (address - IO_START) as usize;
        let end = start + width;
        if start < 8 && end > 4 {
            let status = self.display_status();
            self.io[4..8].copy_from_slice(&status);
        }
        if start < 0xa0 && end > 0x60 {
            self.audio.refresh_controls(&mut self.io);
        }
        if start < 0x110 && end > 0x100 {
            self.audio
                .refresh_timers_at(&mut self.io, self.cycles - self.audio_at);
        }
        if start < 0x132 && end > 0x130 {
            self.io[0x130..0x132].copy_from_slice(&(!self.buttons.0 & 0x03ff).to_le_bytes());
        }
    }

    /// A diagnostic snapshot projects counters without changing timing, IF or
    /// FIFO state. Live accesses instead materialize their own register family.
    pub(super) fn refresh_readback(&self, io: &mut [u8]) {
        io[4..8].copy_from_slice(&self.display_status());
        self.audio
            .refresh_timers_at(io, self.cycles - self.audio_at);
        self.audio.refresh_controls(io);
        io[0x130..0x132].copy_from_slice(&(!self.buttons.0 & 0x03ff).to_le_bytes());
    }

    /// Fetch from the current executable region with the ordinary timing path.
    /// BIOS protection, GPIO overlays and unusual executable regions keep the
    /// general handler. ROM bounds are checked independently of region caching.
    fn fetch_impl(&mut self, address: u32, width: usize, access: Access) -> Result<u32, CoreError> {
        let tag = address >> 24;
        if self.fetch_tag != tag {
            self.fetch_tag = tag;
            self.fetch_region = match tag {
                0x02 => FetchRegion::Ewram,
                0x03 => FetchRegion::Iwram,
                0x08..=0x0d => FetchRegion::Rom,
                _ => FetchRegion::Other,
            };
        }
        if matches!(self.fetch_region, FetchRegion::Other)
            || (self.rtc.is_some() && (0x080000c4..0x080000cc).contains(&address))
        {
            return if width == 2 {
                self.read16_impl(address, access).map(u32::from)
            } else {
                self.read32_impl(address, access)
            };
        }
        if address as usize & (width - 1) != 0 {
            return Err(CoreError::InvalidAccessAlignment { address, width });
        }
        self.charge(address, width, access);
        let (backing, offset) = match self.fetch_region {
            FetchRegion::Ewram => (&self.ewram, address as usize & (self.ewram.len() - 1)),
            FetchRegion::Iwram => (&self.iwram, address as usize & (self.iwram.len() - 1)),
            FetchRegion::Rom => (&self.rom, address as usize & 0x01ff_ffff),
            FetchRegion::Other => unreachable!("fallback fetch handled before timing"),
        };
        let bytes = backing
            .get(offset..offset + width)
            .ok_or(CoreError::UnmappedAddress { address, width })?;
        let value = if width == 2 {
            u32::from(u16::from_le_bytes([bytes[0], bytes[1]]))
        } else {
            u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
        };
        self.open_bus = if width == 2 {
            value * 0x0001_0001
        } else {
            value
        };
        Ok(value)
    }

    pub(super) fn read16_impl(&mut self, address: u32, access: Access) -> Result<u16, CoreError> {
        if address & 1 != 0 {
            return Err(CoreError::InvalidAccessAlignment { address, width: 2 });
        }
        self.charge(address, 2, access);
        let region = address >> 24;
        if matches!(region, 0x08..=0x0d)
            && let Some(value) = self.gpio_read(address)
        {
            self.open_bus = u32::from(value) * 0x0001_0001;
            return Ok(value);
        }
        if region == 0x0d
            && self.eeprom_address(address)
            && matches!(access.kind, AccessKind::Data)
            && let Some(eeprom) = self.eeprom.as_mut()
        {
            #[cfg(feature = "eeprom-trace")]
            eeprom.trace_cycle(self.cycles);
            let value = eeprom.read();
            self.open_bus = u32::from(value) * 0x0001_0001;
            return Ok(value);
        }
        if matches!(region, 0x0e..=0x0f)
            && let Some(value) = self.backup_read(address)
        {
            let value = u16::from(value) * 0x0101;
            self.open_bus = u32::from(value) * 0x0001_0001;
            return Ok(value);
        }
        if region == 0
            && let Some(word) = self.bios_read(address, access)
        {
            return Ok((word >> ((address & 2) * 8)) as u16);
        }
        let value = match self.read_bytes(address, 2) {
            Ok(bytes) => u16::from_le_bytes([bytes[0], bytes[1]]),
            Err(_) if matches!(access.kind, AccessKind::Data) => {
                (self.unmapped_read() >> ((address & 2) * 8)) as u16
            }
            Err(error) => return Err(error),
        };
        self.open_bus = u32::from(value) * 0x0001_0001;
        Ok(value)
    }

    pub(super) fn read32_impl(&mut self, address: u32, access: Access) -> Result<u32, CoreError> {
        if address & 3 != 0 {
            return Err(CoreError::InvalidAccessAlignment { address, width: 4 });
        }
        self.charge(address, 4, access);
        let region = address >> 24;
        if matches!(region, 0x08..=0x0d) && self.rtc.is_some() {
            let low = self.gpio_read(address);
            let high = self.gpio_read(address + 2);
            if low.is_some() || high.is_some() {
                let half = |a, gpio: Option<u16>| {
                    gpio.unwrap_or_else(|| {
                        self.read_bytes(a, 2)
                            .map_or(0, |b| u16::from_le_bytes([b[0], b[1]]))
                    })
                };
                let value =
                    u32::from(half(address, low)) | (u32::from(half(address + 2, high)) << 16);
                self.open_bus = value;
                return Ok(value);
            }
        }
        if matches!(region, 0x0e..=0x0f)
            && let Some(value) = self.backup_read(address)
        {
            self.open_bus = u32::from(value) * 0x0101_0101;
            return Ok(self.open_bus);
        }
        if region == 0
            && let Some(word) = self.bios_read(address, access)
        {
            return Ok(word);
        }
        let value = match self.read_bytes(address, 4) {
            Ok(bytes) => u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            Err(_) if matches!(access.kind, AccessKind::Data) => self.unmapped_read(),
            Err(error) => return Err(error),
        };
        self.open_bus = value;
        Ok(value)
    }

    pub(super) fn write16_impl(
        &mut self,
        address: u32,
        value: u16,
        access: Access,
    ) -> Result<(), CoreError> {
        // Backup memory is an eight-bit bus: retain the original address to
        // select the corresponding source byte even for an unaligned store.
        if (0x0e000000..0x10000000).contains(&address)
            && (self.sram.is_some() || self.flash.is_some())
        {
            self.charge(address, 2, access);
            self.backup_write(address, value.rotate_right((address & 1) * 8) as u8);
            return Ok(());
        }
        if address & 1 != 0 {
            return Err(CoreError::InvalidAccessAlignment { address, width: 2 });
        }

        self.charge(address, 2, access);

        self.write_halfword(address, value)
    }

    /// Overlay GPIO only for an enabled clock cartridge; other reads retain ROM.
    fn gpio_read(&self, address: u32) -> Option<u16> {
        self.rtc.as_ref().and_then(|chip| chip.read(address))
    }

    /// Apply one already-timed bus beat, including cartridge peripherals.
    pub(super) fn write_halfword(&mut self, address: u32, value: u16) -> Result<(), CoreError> {
        if self
            .rtc
            .as_mut()
            .is_some_and(|chip| chip.write(address, value))
        {
            return Ok(());
        }
        self.synchronize_io_write(address);
        // BIOS and the remainder of regions 00h/01h are not writable memory.
        // The enclosing bus operation has already charged its timing.
        if address < EWRAM_START {
            return Ok(());
        }

        if self.eeprom_address(address) {
            if let Some(eeprom) = self.eeprom.as_mut() {
                #[cfg(feature = "eeprom-trace")]
                eeprom.trace_cycle(self.cycles);
                eeprom.write(value);
            }

            return Ok(());
        }

        let bytes = value.to_le_bytes();

        if let Some(range) = ram_range(address, EWRAM_START, self.ewram.len(), 2) {
            self.ewram[range].copy_from_slice(&bytes);
            return Ok(());
        }

        if let Some(range) = ram_range(address, IWRAM_START, self.iwram.len(), 2) {
            self.iwram[range].copy_from_slice(&bytes);
            return Ok(());
        }

        if let Some(range) = range_for(address, IO_START, self.io.len(), 2) {
            let offset = range.start;
            if matches!(offset, 0x50 | 0x52 | 0x54) {
                let stored = if offset == 0x50 {
                    // BLDCNT: bits 14..15 are unused.
                    value & 0x3fff
                } else if offset == 0x52 {
                    // BLDALPHA: EVA and EVB are five-bit register fields.
                    value & 0x1f1f
                } else {
                    // BLDY: effective EVY saturates at 16.
                    (value & 0x001f).min(16)
                };

                self.io[range].copy_from_slice(&stored.to_le_bytes());
                return Ok(());
            }

            if matches!(offset, 0xba | 0xc6 | 0xd2 | 0xde) {
                let channel = (offset - 0xba) / 12;
                // Bits 0..4 are unused on every channel; Game Pak DRQ is DMA3-only.
                let control = value & if channel < 3 { 0xf7e0 } else { 0xffe0 };
                self.io[range].copy_from_slice(&control.to_le_bytes());
                self.configure_dma(channel, control);
                return Ok(());
            }
            if (0x100..0x110).contains(&offset) {
                self.audio.write_timer(offset, value);
                self.audio.refresh_timers(&mut self.io);
                return Ok(());
            }
            if matches!(
                offset,
                0x60 | 0x62 | 0x64 | 0x68 | 0x6c | 0x70 | 0x72 | 0x74 | 0x78 | 0x7c | 0x90
                    ..=0x9e | 0x80 | 0x82 | 0x84 | 0x88
            ) {
                self.audio.write_control(offset, value);
                if offset == 0x84 && value & 0x80 == 0 {
                    self.io[0x60..0x82].fill(0);
                }
                self.audio.refresh_controls(&mut self.io);
                return Ok(());
            }
            if matches!(offset, 0xa0 | 0xa2 | 0xa4 | 0xa6) {
                self.audio.push_channel(usize::from(offset >= 0xa4), &bytes);
                return Ok(());
            }
            if matches!(offset, 6 | 0x86 | 0x8a | 0x130) {
                return Ok(());
            }
            if offset == 0x132 {
                self.io[range].copy_from_slice(&(value & 0xc3ff).to_le_bytes());
                self.evaluate_keypad();
                return Ok(());
            }
            if matches!(offset, 0x200 | 0x202 | 0x208) {
                let old = u16::from_le_bytes([self.io[offset], self.io[offset + 1]]);
                let value = match offset {
                    0x202 => old & !(value & 0x3fff),
                    0x208 => value & 1,
                    _ => value & 0x3fff,
                };
                self.io[range].copy_from_slice(&value.to_le_bytes());
                return Ok(());
            }
            if offset == 0x300 {
                self.io[0x300] = value as u8;
                self.halted = value & 0x8000 == 0;
                return Ok(());
            }
            if offset == 0x204 {
                self.gamepak.head = None;
                self.gamepak.buffered = 0;
                self.gamepak.progress = 0;
                self.io[range].copy_from_slice(&(value & 0x5fff).to_le_bytes());
                self.waitstates = Waitstates::from_control(value & 0x5fff);
                return Ok(());
            }
            self.io[range].copy_from_slice(&bytes);
            self.display.write_reference(offset, &self.io);
            if offset == 4 {
                self.refresh_status();
            }

            if offset == 0 {
                self.display.control = value;
            }

            return Ok(());
        }

        if let Some(range) = palette_range(address, 2) {
            self.palette[range].copy_from_slice(&bytes);
            return Ok(());
        }

        if let Some(range) = vram_range(address, 2) {
            self.vram[range].copy_from_slice(&bytes);
            return Ok(());
        }

        if let Some(range) = ram_range(address, OAM_START, OAM_BYTES, 2) {
            self.oam[range].copy_from_slice(&bytes);
            return Ok(());
        }

        // Ordinary writes to cartridge ROM space do not trap on the GBA.
        // Cartridge peripherals such as EEPROM must intercept their addresses
        // before reaching this fallback. Future GPIO/RTC handling must do the same.
        if (0x0800_0000..0x0e00_0000).contains(&address) {
            return Ok(());
        }

        Err(CoreError::UnmappedAddress { address, width: 2 })
    }

    pub(super) fn write32_impl(
        &mut self,
        address: u32,
        value: u32,
        access: Access,
    ) -> Result<(), CoreError> {
        // Backup memory is an eight-bit bus: retain the original address to
        // select the corresponding source byte even for an unaligned store.
        if (0x0e000000..0x10000000).contains(&address)
            && (self.sram.is_some() || self.flash.is_some())
        {
            self.charge(address, 4, access);
            self.backup_write(address, value.rotate_right((address & 3) * 8) as u8);
            return Ok(());
        }
        if address & 3 != 0 {
            return Err(CoreError::InvalidAccessAlignment { address, width: 4 });
        }
        self.charge(address, 4, access);

        // An aligned word cannot straddle a physical RAM mirror boundary.
        // Dispatch once, retaining the same bus timing as two halfword beats.
        // Scanout edges crossed by charge() have already observed the old bytes.
        let bytes = value.to_le_bytes();
        match address >> 24 {
            0x02 => {
                let offset = address as usize & (self.ewram.len() - 1);
                self.ewram[offset..offset + 4].copy_from_slice(&bytes);
                return Ok(());
            }
            0x03 => {
                let offset = address as usize & (self.iwram.len() - 1);
                self.iwram[offset..offset + 4].copy_from_slice(&bytes);
                return Ok(());
            }
            0x05 => {
                let offset = address as usize & (PALETTE_BYTES - 1);
                self.palette[offset..offset + 4].copy_from_slice(&bytes);
                return Ok(());
            }
            0x06 => {
                let mut offset = address as usize & 0x1ffff;
                if offset >= 0x18000 {
                    offset -= 0x8000;
                }
                self.vram[offset..offset + 4].copy_from_slice(&bytes);
                return Ok(());
            }
            0x07 => {
                let offset = address as usize & (OAM_BYTES - 1);
                self.oam[offset..offset + 4].copy_from_slice(&bytes);
                return Ok(());
            }
            0x04 if matches!(address, 0x040000a0 | 0x040000a4) => {
                self.synchronize_io_write(address);
                self.audio
                    .push_channel(usize::from(address == IO_START + 0xa4), &bytes);
                return Ok(());
            }
            _ => {}
        }
        // Register masks, DMA enable edges, GPIO and serial cartridge devices
        // retain their existing low-halfword-first semantics at one timestamp.
        self.write_halfword(address, value as u16)?;
        self.write_halfword(address.wrapping_add(2), (value >> 16) as u16)
    }

    /// EEPROM uses all of region 0D for ROMs up to 16 MiB, and only the
    /// final 256 bytes for larger cartridges. Other ROM mirrors remain intact.
    fn eeprom_address(&self, address: u32) -> bool {
        self.eeprom.is_some()
            && (0x0d000000..0x0e000000).contains(&address)
            && (self.rom.len() <= 0x01000000 || address >= 0x0dffff00)
    }

    /// The eight-bit save bus delegates mirroring and commands to the selected chip.
    fn backup_read(&mut self, address: u32) -> Option<u8> {
        if !(0x0e000000..0x10000000).contains(&address) {
            return None;
        }
        self.sram
            .as_ref()
            .map(|sram| sram.read(address))
            .or_else(|| self.flash.as_mut().map(|flash| flash.read(address)))
    }

    pub(super) fn backup_write(&mut self, address: u32, value: u8) -> bool {
        if !(0x0e000000..0x10000000).contains(&address) {
            return false;
        }
        if let Some(sram) = &mut self.sram {
            sram.write(address, value);
            true
        } else if let Some(flash) = &mut self.flash {
            flash.write(address, value);
            true
        } else {
            false
        }
    }

    /// DMA observes the data bus; CPU unmapped reads observe instruction prefetch.
    fn unmapped_read(&self) -> u32 {
        if self.dma.iter().any(|dma| dma.active) {
            self.open_bus
        } else {
            self.cpu_open_bus
        }
    }

    /// BIOS protection depends on CPU execution context, including DMA reads.
    /// Width-specific callers select the corresponding byte lanes from this word.
    fn bios_read(&mut self, address: u32, access: Access) -> Option<u32> {
        if !self.bios_enabled {
            return None;
        }
        let bios = self.bios.as_ref()?;
        if address >= BIOS_SIZE as u32 {
            return None;
        }
        if self.execution_address < BIOS_SIZE as u32 {
            let offset = address as usize & !3;
            let word = u32::from_le_bytes([
                bios[offset],
                bios[offset + 1],
                bios[offset + 2],
                bios[offset + 3],
            ]);
            if matches!(access.kind, AccessKind::Fetch) {
                self.bios_latch = word;
            }
            Some(word)
        } else {
            Some(self.bios_latch)
        }
    }

    pub(super) fn read_bytes(&self, address: u32, width: usize) -> Result<&[u8], CoreError> {
        // Decode the bus region once, retaining physical bounds and mirror
        // rules. Out-of-range ROM accesses keep the existing open-bus policy.
        let bytes = match address >> 24 {
            0x00 if self.test_firmware => {
                range_for(address, 0, TEST_FIRMWARE.len(), width).map(|range| &TEST_FIRMWARE[range])
            }
            0x02 => ram_range(address, EWRAM_START, self.ewram.len(), width)
                .map(|range| &self.ewram[range]),
            0x03 => ram_range(address, IWRAM_START, self.iwram.len(), width)
                .map(|range| &self.iwram[range]),
            0x04 => range_for(address, IO_START, self.io.len(), width).map(|range| &self.io[range]),
            0x05 => palette_range(address, width).map(|range| &self.palette[range]),
            0x06 => vram_range(address, width).map(|range| &self.vram[range]),
            0x07 => ram_range(address, OAM_START, OAM_BYTES, width).map(|range| &self.oam[range]),
            0x08..=0x0d => range_for(address & 0x01ffffff, 0, self.rom.len(), width)
                .map(|range| &self.rom[range]),
            _ => None,
        };
        if let Some(bytes) = bytes {
            return Ok(bytes);
        }
        Err(CoreError::UnmappedAddress { address, width })
    }

    /// Charges timing at the bus boundary, once per CPU access. Internal memory
    /// leaves the cartridge free to prefetch; SRAM and ROM accesses own that bus.
    pub(super) fn charge(&mut self, address: u32, width: usize, access: Access) {
        let beats = (width / 2).max(1) as u64;
        let cartridge = (0x08000000..0x10000000).contains(&address);
        let cycles = match address >> 24 {
            0x08..=0x0d => self
                .gamepak
                .access(&self.waitstates, address, width, access),
            0x0e..=0x0f => {
                self.gamepak = GamePak::default();
                self.waitstates.sram
            }
            0x02 => 3 * beats,
            0x03 | 0x07 => 1,
            0x04 if address - IO_START < IO_BYTES as u32 => 1,
            _ => beats,
        };
        if !cartridge {
            self.gamepak.fill(&self.waitstates, cycles);
        }
        self.advance_time(cycles);
        if address >> 24 == 0x04 {
            // Dynamic readbacks are projected without advancing devices again.
            // Actual register mutations synchronize and invalidate separately.
            self.refresh_io_access(address, width);
        }
    }

    /// Advances display events before any access changes the state observed by scanout.
    pub(super) fn advance_time(&mut self, cycles: u64) {
        let deadline = self.cycles.saturating_add(cycles);
        if self.next_device_event <= self.cycles {
            self.synchronize_devices();
        }
        while self.next_device_event <= deadline {
            self.cycles = self.next_device_event;
            self.synchronize_devices();
        }
        self.cycles = deadline;
    }

    /// Materializes deferred time before a device observation or mutation.
    /// All event calculations use the previous device clock, so IRQ/DMA edges
    /// cannot be skipped when multiple bus accesses share one cached deadline.
    fn synchronize_devices(&mut self) {
        let target = self.cycles;
        let next_video_event = if target != self.devices_at
            || self
                .inputs
                .front()
                .is_some_and(|event| event.cycle.0 <= target)
        {
            self.cycles = self.devices_at;
            let next = self.advance_devices(target);
            self.devices_at = target;
            next
        } else {
            self.next_wake_event()
        };
        // A zero-time invalidation only refreshes deadlines. Device phases,
        // scanout and DMA request levels need no second materialization.
        self.next_device_event = self
            .audio
            .next_event(self.audio_at)
            .min(self.display.next_event)
            .min(next_video_event);
        if let Some(event) = self.inputs.front() {
            self.next_device_event = self.next_device_event.min(event.cycle.0);
        }
    }

    /// Materialize only state required by the register being changed. Ordinary
    /// display registers observe already-processed scanout edges; audio owns a
    /// separate clock. Only controls that alter scheduled edges invalidate them.
    fn synchronize_io_write(&mut self, address: u32) {
        if address >> 24 != 0x04 {
            return;
        }
        self.cpu_boundary = true;
        let offset = (address - IO_START) & !1;
        let audio = matches!(offset, 0x60..=0x88 | 0x90..=0xa6 | 0x100..=0x10e);
        let timed_video = matches!(offset, 4 | 0xba | 0xc6 | 0xd2 | 0xde);
        if audio {
            self.advance_audio_to(self.cycles);
        } else if timed_video {
            // Reanchor IRQ/DMA edge calculations before applying new controls;
            // enabling an edge must never request it retroactively.
            self.synchronize_devices();
        }
        if timed_video || (audio && !(0xa0..=0xa6).contains(&offset)) {
            self.next_device_event = self.cycles;
        }
    }

    /// Advance audio under the old configuration before a register mutation.
    /// Bus timing has already processed intervening sample/timer/oscillator
    /// edges, so this interval contains no unprocessed hardware boundary.
    fn advance_audio_to(&mut self, target: u64) {
        if target == self.audio_at {
            return;
        }
        let (irq, refill) = self.audio.advance(target - self.audio_at, target);
        self.audio_at = target;
        let flags = u16::from_le_bytes([self.io[0x202], self.io[0x203]]) | irq;
        self.io[0x202..0x204].copy_from_slice(&flags.to_le_bytes());
        for dma in &mut self.dma[1..=2] {
            if refill[usize::from(dma.destination == IO_START + 0xa4)]
                && dma.control & 0xb000 == 0xb000
                && matches!(dma.destination, 0x040000a0 | 0x040000a4)
                && !dma.active
            {
                dma.active = true;
            }
        }
    }

    /// Materializes device edges and returns the next required video edge.
    /// Future edges remain valid throughout this call: only DMA request levels
    /// and input state change, not DISPSTAT or DMA timing configuration.
    fn advance_devices(&mut self, target: u64) -> u64 {
        // Raise timed requests before CPU execution can continue. Each channel
        // still transfers one bus beat at a time through Machine::step(), where
        // the existing channel order provides hardware priority.
        let vblank = self.next_vblank();
        let hblank = self
            .dma
            .iter()
            .any(|dma| dma.enabled_for(2))
            .then(|| self.next_visible_hblank());
        // HBlank status/IRQ edges continue on blank scanlines; ordinary
        // HBlank DMA remains restricted to visible lines.
        let hblank_irq = (self.io[4] & 0x10 != 0).then(|| self.next_hblank());
        let vcount = if self.io[4] & 0x20 != 0 {
            self.next_vcount_match()
        } else {
            None
        };

        let vblank_due = vblank <= target;
        let hblank_due = hblank.is_some_and(|edge| edge <= target);
        let hblank_irq_due = hblank_irq.is_some_and(|edge| edge <= target);
        let vcount_due = vcount.is_some_and(|edge| edge <= target);

        for dma in &mut self.dma {
            if dma.active {
                continue;
            }

            if (vblank_due && dma.enabled_for(1)) || (hblank_due && dma.enabled_for(2)) {
                dma.active = true;
            }
        }

        // Repeated reads in VBlank must not regenerate an acknowledged request.
        if vblank_due && self.io[4] & 8 != 0 {
            self.io[0x202] |= 1;
        }
        if hblank_irq_due {
            self.io[0x202] |= 2;
        }

        // VCOUNT requests IRQ exactly when VCOUNT enters the programmed comparison
        // scanline. IF retains the request until guest software acknowledges bit 2.
        if vcount_due && self.io[4] & 0x20 != 0 {
            self.io[0x202] |= 1 << 2;
        }
        while self
            .inputs
            .front()
            .is_some_and(|event| event.cycle.0 <= target)
        {
            let Some(event) = self.inputs.pop_front() else {
                break;
            };

            if self.buttons.set(event.button, event.pressed) {
                self.evaluate_keypad();
            }
        }
        self.display
            .synchronize_to(target, &self.vram, &self.palette, &self.io, &self.oam);
        self.advance_audio_to(target);
        self.cycles = target;
        // Reuse every edge that has not crossed. Recalculate only a consumed
        // edge against the new device clock, retaining exact IRQ/DMA boundaries.
        let mut next = if vblank_due {
            self.next_vblank()
        } else {
            vblank
        };
        if let Some(edge) = hblank {
            next = next.min(if hblank_due {
                self.next_visible_hblank()
            } else {
                edge
            });
        }
        if let Some(edge) = hblank_irq {
            next = next.min(if hblank_irq_due {
                self.next_hblank()
            } else {
                edge
            });
        }
        if let Some(edge) = if vcount_due {
            self.next_vcount_match()
        } else {
            vcount
        } {
            next = next.min(edge);
        }
        next
    }
}

impl CpuBus for System {
    fn fetch16(&mut self, address: u32, access: Access) -> Result<u16, CoreError> {
        self.fetch_impl(address, 2, access)
            .map(|value| value as u16)
    }

    fn fetch32(&mut self, address: u32, access: Access) -> Result<u32, CoreError> {
        self.fetch_impl(address, 4, access)
    }

    fn set_cpu_open_bus(&mut self, pipeline: [u32; 2], address: u32, thumb: bool) {
        self.cpu_open_bus = if !thumb {
            pipeline[1]
        } else {
            let fetch_address = address.wrapping_add(4);
            match fetch_address >> 24 {
                // These regions have a 32-bit instruction bus. BIOS fetches
                // already captured the complete aligned word in the protected latch.
                0 if self.bios_enabled => self.bios_latch,
                7 => self
                    .read_bytes(fetch_address & !3, 4)
                    .map(|bytes| u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
                    .unwrap_or(pipeline[1] * 0x0001_0001),
                3 if fetch_address & 2 != 0 => (pipeline[1] << 16) | pipeline[0],
                3 => pipeline[1] | (pipeline[0] << 16),
                _ => pipeline[1] * 0x00010001,
            }
        };
    }

    fn set_execution_address(&mut self, address: u32) {
        self.execution_address = address;
    }
    fn restart_fetch(&mut self) {
        self.gamepak = GamePak::default();
    }

    fn read8(&mut self, address: u32, access: Access) -> Result<u8, CoreError> {
        self.charge(address, 1, access);
        if let Some(value) = self.gpio_read(address & !1) {
            return Ok((value >> ((address & 1) * 8)) as u8);
        }
        if let Some(value) = self.backup_read(address) {
            return Ok(value);
        }
        if let Some(word) = self.bios_read(address, access) {
            return Ok((word >> ((address & 3) * 8)) as u8);
        }
        match self.read_bytes(address, 1) {
            Ok(bytes) => Ok(bytes[0]),
            Err(_) if matches!(access.kind, AccessKind::Data) => {
                Ok((self.unmapped_read() >> ((address & 3) * 8)) as u8)
            }
            Err(error) => Err(error),
        }
    }

    /// Video memory has a 16-bit write bus: palette and BG bytes are duplicated,
    /// while OBJ VRAM and OAM ignore byte stores. RAM bytes retain ordinary semantics.
    fn write8(&mut self, address: u32, value: u8, access: Access) -> Result<(), CoreError> {
        self.charge(address, 1, access);
        self.synchronize_io_write(address);
        if self.backup_write(address, value) {
            return Ok(());
        }

        // Region 00h/01h contains the BIOS followed by unmapped system space.
        // Stores consume bus time but have no writable target. Real hardware does
        // not turn these accesses into a guest-visible execution failure.
        if address < EWRAM_START {
            return Ok(());
        }

        // RegisterRamReset writes a byte to this undocumented write-only
        // location. Accept the timed access without backing unused MMIO space;write8(
        // neighboring addresses retain their existing unmapped behavior.
        if address == BIOS_UNDOCUMENTED_IO_410 {
            return Ok(());
        }
        if let Some(range) = ram_range(address, EWRAM_START, self.ewram.len(), 1) {
            self.ewram[range.start] = value;
        } else if let Some(range) = ram_range(address, IWRAM_START, self.iwram.len(), 1) {
            self.iwram[range.start] = value;
        } else if let Some(range) = range_for(address, IO_START, self.io.len(), 1) {
            let offset = range.start;
            if (0xa0..0xa8).contains(&offset) {
                self.audio
                    .push_channel(usize::from(offset >= 0xa4), &[value]);
            } else if offset == 0x301 {
                // STOP remains outside this fixture; HALT uses bit 7 clear.
                self.halted = value & 0x80 == 0;
            } else if offset == 0x300 {
                self.io[offset] = value;
            } else {
                let aligned = offset & !1;
                let shift = (offset & 1) * 8;
                let old = self.audio.pulse_latch(aligned).unwrap_or_else(|| {
                    u16::from_le_bytes([self.io[aligned], self.io[aligned + 1]])
                });
                let merged = if aligned == 0x202 {
                    u16::from(value) << shift
                } else {
                    (old & !(0xff << shift)) | (u16::from(value) << shift)
                };
                self.write_halfword(address & !1, merged)?;
            }
        } else if palette_range(address, 1).is_some() {
            self.write_halfword(address & !1, u16::from(value) * 0x0101)?;
        } else if let Some(range) = vram_range(address, 1) {
            let object_start = if self.display.control & 7 >= 3 {
                0x14000
            } else {
                0x10000
            };
            if range.start < object_start {
                self.write_halfword(address & !1, u16::from(value) * 0x0101)?;
            }
        } else if (0x0800_0000..0x0e00_0000).contains(&address) {
            // ROM bus store with no selected cartridge peripheral.
            // The access consumes timing but has no storage effect.
            return Ok(());
        } else if ram_range(address, OAM_START, OAM_BYTES, 1).is_none() {
            return Err(CoreError::UnmappedAddress { address, width: 1 });
        }
        // OAM has a 32-bit bus but no byte-write enables: STRB is ignored,
        // including mirrors. Halfword and word accesses retain their values.
        Ok(())
    }

    fn read16(&mut self, address: u32, access: Access) -> Result<u16, CoreError> {
        self.read16_impl(address, access)
    }
    fn read32(&mut self, address: u32, access: Access) -> Result<u32, CoreError> {
        self.read32_impl(address, access)
    }

    fn write16(&mut self, address: u32, value: u16, access: Access) -> Result<(), CoreError> {
        self.write16_impl(address, value, access)
    }

    fn write32(&mut self, address: u32, value: u32, access: Access) -> Result<(), CoreError> {
        self.write32_impl(address, value, access)
    }

    fn idle(&mut self, cycles: u64) {
        self.gamepak.fill(&self.waitstates, cycles);
        self.advance_time(cycles);
    }
}

impl Default for System {
    fn default() -> Self {
        Self::new()
    }
}

/// VRAM repeats every 128 KiB; the last 32 KiB mirrors physical 64..96 KiB.
/// Decode before byte-store rules so mirrored OBJ addresses are also ignored.
fn vram_range(address: u32, width: usize) -> Option<Range<usize>> {
    if address >> 24 != VRAM_START >> 24 {
        return None;
    }
    let mut offset = address as usize & 0x1ffff;
    if offset >= 0x18000 {
        offset -= 0x8000;
    }
    (offset + width <= VRAM_BYTES).then_some(offset..offset + width)
}

/// Palette RAM repeats throughout its 16 MiB bus region. Access widths and
/// alignment are checked by the caller before applying the physical RAM mask.
fn palette_range(address: u32, width: usize) -> Option<Range<usize>> {
    if address >> 24 != PALETTE_START >> 24 {
        return None;
    }
    let start = address as usize & (PALETTE_BYTES - 1);
    (start + width <= PALETTE_BYTES).then_some(start..start + width)
}

/// Work RAM repeats throughout its region; decode before accessing backing storage.
fn ram_range(address: u32, start: u32, length: usize, width: usize) -> Option<Range<usize>> {
    if address >> 24 != start >> 24 {
        return None;
    }
    let offset = address as usize & (length - 1);
    (offset.checked_add(width)? <= length).then_some(offset..offset + width)
}

pub(super) fn range_for(
    address: u32,
    start: u32,
    length: usize,
    width: usize,
) -> Option<Range<usize>> {
    let offset = address.checked_sub(start)? as usize;
    let end = offset.checked_add(width)?;
    (end <= length).then_some(offset..end)
}

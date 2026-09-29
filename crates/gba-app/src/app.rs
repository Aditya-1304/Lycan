use eframe::egui;
use gba_session::{
    BUTTONS, Button, CYCLES_PER_FRAME, GBA_CLOCK_HZ, SCREEN_HEIGHT, SCREEN_WIDTH, Session,
};
use std::time::Duration;
use web_time::Instant;

#[cfg(target_arch = "wasm32")]
type PendingRom = Option<(String, Result<Vec<u8>, String>)>;

const WIDTH: usize = SCREEN_WIDTH;
const HEIGHT: usize = SCREEN_HEIGHT;
const WAKE_SECONDS: f64 = CYCLES_PER_FRAME as f64 / GBA_CLOCK_HZ as f64;

const PIXELS_ROM: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../roms/pixels.gba"
));

/// Keyboard bindings translate host keys into the session's platform-neutral buttons.
const KEY_BINDINGS: [(Button, egui::Key); 10] = [
    (Button::A, egui::Key::Z),
    (Button::B, egui::Key::X),
    (Button::L, egui::Key::A),
    (Button::R, egui::Key::S),
    (Button::Start, egui::Key::Enter),
    (Button::Select, egui::Key::Backspace),
    (Button::Up, egui::Key::ArrowUp),
    (Button::Down, egui::Key::ArrowDown),
    (Button::Left, egui::Key::ArrowLeft),
    (Button::Right, egui::Key::ArrowRight),
];

/// Shared native/browser application displaying pixels produced by guest execution.
pub struct GbaApp {
    session: Session,
    rom_name: String,
    loaded: bool,
    next_wake: f64,
    last_guest_generation: u64,
    core_times: Measurements,
    conversion_times: Measurements,
    upload_times: Measurements,
    screen_image: egui::ColorImage,
    texture: Option<egui::TextureHandle>,
    image_generation: u64,
    uploaded_generation: Option<u64>,
    status: String,
    // Optional development capture uses eframe's actual rendered native viewport.
    #[cfg(not(target_arch = "wasm32"))]
    capture_path: Option<std::path::PathBuf>,
    #[cfg(not(target_arch = "wasm32"))]
    capture_requested: bool,
    #[cfg(target_arch = "wasm32")]
    pending_rom: std::rc::Rc<std::cell::RefCell<PendingRom>>,
    #[cfg(target_arch = "wasm32")]
    load_generation: std::rc::Rc<std::cell::Cell<u64>>,
}

impl GbaApp {
    /// Creates a session and loads the assembled guest artifact without executing in startup.
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self {
            session: Session::new(),
            rom_name: String::new(),
            loaded: false,
            next_wake: 0.0,
            last_guest_generation: 0,
            core_times: Measurements::default(),
            conversion_times: Measurements::default(),
            upload_times: Measurements::default(),
            screen_image: egui::ColorImage::filled([WIDTH, HEIGHT], egui::Color32::BLACK),
            texture: None,
            image_generation: 0,
            uploaded_generation: None,
            status: "Loading pixels.gba".to_owned(),
            #[cfg(not(target_arch = "wasm32"))]
            capture_path: std::env::var_os("GBA_CAPTURE_PATH").map(Into::into),
            #[cfg(not(target_arch = "wasm32"))]
            capture_requested: false,
            #[cfg(target_arch = "wasm32")]
            pending_rom: Default::default(),
            #[cfg(target_arch = "wasm32")]
            load_generation: Default::default(),
        };
        app.load_rom_bytes("pixels.gba", PIXELS_ROM);
        app
    }

    /// Accepts bytes from the built-in ROM or a native/browser dropped file.
    fn load_rom_bytes(&mut self, name: &str, bytes: &[u8]) {
        match self.session.load_rom(bytes) {
            Ok(()) => {
                self.rom_name = name.to_owned();
                self.loaded = true;
                self.status = format!("Loaded {name}");
                self.next_wake = 0.0;
                self.last_guest_generation = 0;
                self.screen_image.pixels.fill(egui::Color32::BLACK);
                self.image_generation = self.image_generation.wrapping_add(1);
                self.core_times = Measurements::default();
                self.conversion_times = Measurements::default();
                self.upload_times = Measurements::default();
            }
            Err(error) => self.status = format!("ROM load failed: {error}"),
        }
    }

    /// Converts the core's row-major BGR555 pixels into the shared egui image.
    fn copy_framebuffer_to_image(&mut self) {
        for (destination, &pixel) in self
            .screen_image
            .pixels
            .iter_mut()
            .zip(self.session.framebuffer())
        {
            let red = expand_five_bit(pixel & 0x1F);
            let green = expand_five_bit((pixel >> 5) & 0x1F);
            let blue = expand_five_bit((pixel >> 10) & 0x1F);
            *destination = egui::Color32::from_rgb(red, green, blue);
        }
    }

    /// Samples held host keys into the logical button state owned by the session.
    fn poll_keyboard(&mut self, ctx: &egui::Context) {
        ctx.input(|input| {
            for (button, key) in KEY_BINDINGS {
                self.session.set_button(button, input.key_down(key));
            }
        });
    }

    /// Creates one nearest-neighbor texture and reuses it when guest pixels change.
    fn sync_texture(&mut self, ctx: &egui::Context) {
        let start = Instant::now();
        let changed = self.uploaded_generation != Some(self.image_generation);
        if self.texture.is_none() {
            self.texture = Some(ctx.load_texture(
                "gba-framebuffer",
                self.screen_image.clone(),
                egui::TextureOptions::NEAREST,
            ));
            self.uploaded_generation = Some(self.image_generation);
        } else if self.uploaded_generation != Some(self.image_generation) {
            if let Some(texture) = &mut self.texture {
                texture.set(self.screen_image.clone(), egui::TextureOptions::NEAREST);
            }
            self.uploaded_generation = Some(self.image_generation);
        }
        if changed {
            self.upload_times
                .push(start.elapsed().as_secs_f64() * 1000.0);
        }
    }

    /// Resets guest execution while preserving the loaded ROM and resuming wake requests.
    fn reset_demo(&mut self) {
        self.session.reset();
        self.next_wake = 0.0;
        self.last_guest_generation = 0;
        self.screen_image.pixels.fill(egui::Color32::BLACK);
        self.status = format!("Reset {}", self.rom_name);
        self.image_generation = self.image_generation.wrapping_add(1);
    }

    /// Draws controls, guest status, input state, and the aspect-preserving framebuffer.
    fn draw_ui(&mut self, ui: &mut egui::Ui) {
        self.sync_texture(ui.ctx());
        #[cfg(not(target_arch = "wasm32"))]
        if self.capture_path.is_some() && !self.capture_requested && self.core_times.count == 120 {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
            self.capture_requested = true;
        }

        ui.heading("gba-rs");
        ui.horizontal(|ui| {
            let pause_label = if self.session.paused() {
                "Resume"
            } else {
                "Pause"
            };

            if ui.button(pause_label).clicked() {
                self.session.toggle_pause();
                self.next_wake = 0.0;
                ui.ctx().request_repaint();
            }
            if ui.button("Reset").clicked() {
                self.reset_demo();
                ui.ctx().request_repaint();
            }

            ui.label(format!(
                "Instructions: {} | GBA cycles: {}",
                self.session.executed_instructions(),
                self.session.cycles().0
            ));
            ui.label(if self.session.paused() {
                "Paused"
            } else {
                "Running"
            });
        });

        ui.label(&self.status);
        ui.label("Drop a .gba ROM here to load it.");
        ui.collapsing("Performance", |ui| {
            ui.label(self.core_times.label("Core execution"));
            ui.label(self.conversion_times.label("Pixel conversion"));
            ui.label(self.upload_times.label("Texture submission"));
            ui.small(
                "Texture submission measures CPU queue/copy time; GPU completion is not timed.",
            );
        });
        ui.separator();
        ui.label("Logical buttons");
        ui.horizontal_wrapped(|ui| {
            for button in BUTTONS {
                let state = if self.session.button_pressed(button) {
                    "DOWN"
                } else {
                    "UP"
                };
                ui.monospace(format!("{}: {state}", button_name(button)));
            }
        });
        ui.label("Keys: Z=A, X=B, A=L, S=R, Enter=Start, Backspace=Select, arrows=direction");

        ui.separator();
        if let Some(texture) = &self.texture {
            let size = fitted_screen_size(ui.available_size());
            ui.add(egui::Image::from_texture(texture).fit_to_exact_size(size));
        }
    }
}

impl eframe::App for GbaApp {
    /// Executes bounded guest work while running and schedules the next host wake.
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        #[cfg(not(target_arch = "wasm32"))]
        if self.capture_requested {
            let image = ctx.input(|input| {
                input.events.iter().find_map(|event| {
                    if let egui::Event::Screenshot { image, .. } = event {
                        Some(image.clone())
                    } else {
                        None
                    }
                })
            });
            if let Some(image) = image
                && let Some(path) = self.capture_path.take()
            {
                let mut bytes =
                    format!("P6\n{} {}\n255\n", image.size[0], image.size[1]).into_bytes();
                for pixel in &image.pixels {
                    bytes.extend_from_slice(&pixel.to_array()[..3]);
                }
                let measurements = format!(
                    "{}\n{}\n{}\nframes={}\n",
                    self.core_times.label("Core execution"),
                    self.conversion_times.label("Pixel conversion"),
                    self.upload_times.label("Texture submission"),
                    self.session.framebuffer_generation()
                );
                if let Err(error) = std::fs::write(&path, bytes)
                    .and_then(|()| std::fs::write(path.with_extension("txt"), measurements))
                {
                    eprintln!("Native capture failed: {error}");
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        let dropped = ctx.input(|input| input.raw.dropped_files.clone());
        if let Some(file) = dropped.into_iter().last() {
            let name = file
                .path()
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            #[cfg(not(target_arch = "wasm32"))]
            match file.bytes() {
                Ok(bytes) => self.load_rom_bytes(&name, &bytes),
                Err(error) => self.status = format!("ROM read failed: {error}"),
            }
            #[cfg(target_arch = "wasm32")]
            {
                let generation = self.load_generation.get().wrapping_add(1);
                self.load_generation.set(generation);
                self.pending_rom.borrow_mut().take();
                let current = self.load_generation.clone();
                let pending = self.pending_rom.clone();
                let ctx = ctx.clone();
                self.status = format!("Reading {name}");
                wasm_bindgen_futures::spawn_local(async move {
                    let result = file.bytes_async().await;
                    if current.get() == generation {
                        *pending.borrow_mut() = Some((name, result));
                        ctx.request_repaint();
                    }
                });
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            let completed = self.pending_rom.borrow_mut().take();
            if let Some((name, result)) = completed {
                match result {
                    Ok(bytes) => self.load_rom_bytes(&name, &bytes),
                    Err(error) => self.status = format!("ROM read failed: {error}"),
                }
            }
        }
        if ctx.input(|input| input.focused) {
            self.poll_keyboard(ctx);
        } else {
            self.session.release_all_buttons();
        }
        if !self.loaded || self.session.paused() {
            return;
        }
        let now = ctx.input(|input| input.time);
        if now >= self.next_wake {
            let start = Instant::now();
            match self.session.advance_frame() {
                Ok(_) => {
                    self.core_times.push(start.elapsed().as_secs_f64() * 1000.0);
                    let generation = self.session.framebuffer_generation();
                    if generation != self.last_guest_generation {
                        let start = Instant::now();
                        self.copy_framebuffer_to_image();
                        self.conversion_times
                            .push(start.elapsed().as_secs_f64() * 1000.0);
                        self.last_guest_generation = generation;
                        self.image_generation = self.image_generation.wrapping_add(1);
                    }
                }
                Err(error) => {
                    self.status = format!("Guest execution failed: {error}");
                    self.session.toggle_pause();
                    return;
                }
            }
            // Slice 1 preserves the running wake loop. Elapsed-time/input pacing is Slice 2.
            self.next_wake = now + WAKE_SECONDS;
        }
        ctx.request_repaint_after(Duration::from_secs_f64((self.next_wake - now).max(0.001)));
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.draw_ui(ui);
    }
}

fn expand_five_bit(component: u16) -> u8 {
    let value = component as u8;
    (value << 3) | (value >> 2)
}

/// Returns the largest 3:2 presentation size that fits inside the available UI area.
fn fitted_screen_size(available: egui::Vec2) -> egui::Vec2 {
    let scale = (available.x / WIDTH as f32)
        .min(available.y / HEIGHT as f32)
        .max(0.0);

    egui::vec2(WIDTH as f32 * scale, HEIGHT as f32 * scale)
}

fn button_name(button: Button) -> &'static str {
    match button {
        Button::A => "A",
        Button::B => "B",
        Button::Select => "Select",
        Button::Start => "Start",
        Button::Right => "Right",
        Button::Left => "Left",
        Button::Up => "Up",
        Button::Down => "Down",
        Button::R => "R",
        Button::L => "L",
    }
}

/// Fixed-size samples avoid a growing measurement queue during continued emulation.
struct Measurements {
    values: [f64; 120],
    count: usize,
    next: usize,
}
impl Default for Measurements {
    fn default() -> Self {
        Self {
            values: [0.0; 120],
            count: 0,
            next: 0,
        }
    }
}
impl Measurements {
    fn push(&mut self, milliseconds: f64) {
        self.values[self.next] = milliseconds;
        self.next = (self.next + 1) % self.values.len();
        self.count = (self.count + 1).min(self.values.len());
    }
    fn label(&self, name: &str) -> String {
        if self.count == 0 {
            return format!("{name}: waiting for samples");
        }
        let mean = self.values[..self.count].iter().sum::<f64>() / self.count as f64;
        let mut sorted = self.values;
        sorted[..self.count].sort_by(f64::total_cmp);
        let p95 = sorted[(self.count * 95).div_ceil(100).saturating_sub(1)];
        format!(
            "{name}: mean {mean:.3} ms | p95 {p95:.3} ms | samples {}",
            self.count
        )
    }
}

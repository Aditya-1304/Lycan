use eframe::egui;
use gba_core::fixtures::pixels_gba;
use gba_core::{SCREEN_HEIGHT, SCREEN_WIDTH};
use gba_session::{BUTTONS, Button, Session};

const WIDTH: usize = SCREEN_WIDTH;
const HEIGHT: usize = SCREEN_HEIGHT;
const GUEST_INSTRUCTION_LIMIT: usize = 64;

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
    demo_rom: Vec<u8>,
    screen_image: egui::ColorImage,
    texture: Option<egui::TextureHandle>,
    image_generation: u64,
    uploaded_generation: Option<u64>,
    status: String,
}

impl GbaApp {
    /// Creates a session, executes the built-in guest fixture, and prepares its image.
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self {
            session: Session::new(),
            demo_rom: pixels_gba(),
            screen_image: egui::ColorImage::filled([WIDTH, HEIGHT], egui::Color32::BLACK),
            texture: None,
            image_generation: 0,
            uploaded_generation: None,
            status: "Loading pixels.gba".to_owned(),
        };
        app.load_demo_rom();
        app
    }

    /// Loads the fixture bytes into the core and runs its bounded guest program.
    fn load_demo_rom(&mut self) {
        match self.session.load_rom(&self.demo_rom) {
            Ok(()) => self.execute_guest(),
            Err(error) => self.status = format!("ROM load failed: {error}"),
        }
        self.copy_framebuffer_to_image();
        self.image_generation = self.image_generation.wrapping_add(1);
    }

    /// Executes until the guest reaches its self-branch terminal point.
    fn execute_guest(&mut self) {
        match self.session.run_until_self_branch(GUEST_INSTRUCTION_LIMIT) {
            Ok(report) => {
                self.status = format!("pixels.gba completed at {:#010x}", report.terminal_pc);
            }
            Err(error) => self.status = format!("Guest execution failed: {error}"),
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
    }

    /// Resets the machine, reruns the loaded demo, and refreshes its displayed pixels.
    fn reset_demo(&mut self) {
        self.session.reset();
        self.execute_guest();
        self.copy_framebuffer_to_image();
        self.image_generation = self.image_generation.wrapping_add(1);
    }

    /// Draws controls, guest status, input state, and the aspect-preserving framebuffer.
    fn draw_ui(&mut self, ui: &mut egui::Ui) {
        self.sync_texture(ui.ctx());

        ui.heading("gba-rs");
        ui.horizontal(|ui| {
            let pause_label = if self.session.paused() {
                "Resume"
            } else {
                "Pause"
            };

            if ui.button(pause_label).clicked() {
                self.session.toggle_pause();
            }
            if ui.button("Reset").clicked() {
                self.reset_demo();
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
    /// Keeps logical input current and releases keys when the window loses focus.
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|input| input.focused) {
            self.poll_keyboard(ctx);
        } else {
            self.session.release_all_buttons();
        }
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

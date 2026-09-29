use std::time::Duration;

use eframe::egui;
use gba_session::{BUTTONS, Button, Session};

const WIDTH: usize = 240;
const HEIGHT: usize = 160;
const FRAME_INTERVAL: Duration = Duration::from_millis(16);

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

/// Slice 0 host application with a temporary animated image in place of emulated video.
pub struct GbaApp {
    session: Session,
    test_image: egui::ColorImage,
    texture: Option<egui::TextureHandle>,
    frame: u64,
    uploaded_frame: Option<u64>,
}

impl GbaApp {
    /// Creates a fresh session and its fixed-resolution test image.
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self {
            session: Session::new(),
            test_image: egui::ColorImage::filled([WIDTH, HEIGHT], egui::Color32::BLACK),
            texture: None,
            frame: 0,
            uploaded_frame: None,
        };
        app.render_test_pattern();
        app
    }

    /// Updates the temporary color bars without changing the logical screen dimensions.
    fn render_test_pattern(&mut self) {
        let offset = (self.frame % WIDTH as u64) as usize;

        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let shifted_x = (x + offset) % WIDTH;
                let color = if shifted_x < 80 {
                    egui::Color32::from_rgb(255, 80, 80)
                } else if shifted_x < 160 {
                    egui::Color32::from_rgb(80, 255, 80)
                } else {
                    egui::Color32::from_rgb(80, 80, 255)
                };

                self.test_image.pixels[y * WIDTH + x] = color;
            }
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

    /// Creates one nearest-neighbor texture and reuses it for later image uploads.
    fn sync_texture(&mut self, ctx: &egui::Context) {
        if self.texture.is_none() {
            self.texture = Some(ctx.load_texture(
                "gba-test-framebuffer",
                self.test_image.clone(),
                egui::TextureOptions::NEAREST,
            ));
            self.uploaded_frame = Some(self.frame);
        } else if self.uploaded_frame != Some(self.frame) {
            if let Some(texture) = &mut self.texture {
                texture.set(self.test_image.clone(), egui::TextureOptions::NEAREST);
            }
            self.uploaded_frame = Some(self.frame);
        }
    }

    /// Draws the Slice 0 controls, button state, and aspect-preserving image.
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
                if !self.session.paused() {
                    ui.ctx().request_repaint_after(FRAME_INTERVAL);
                }
            }

            if ui.button("Reset").clicked() {
                self.session.reset();
                self.frame = 0;
                self.render_test_pattern();
                self.uploaded_frame = None;
                ui.ctx().request_repaint_after(FRAME_INTERVAL);
            }

            ui.label(format!("Frame: {}", self.frame));
            ui.label(if self.session.paused() {
                "Paused"
            } else {
                "Running"
            });
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
    /// Advances the temporary frame and schedules another wake only while running.
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|input| input.focused) {
            self.poll_keyboard(ctx);
        } else {
            self.session.release_all_buttons();
        }

        if !self.session.paused() {
            self.frame = self.frame.wrapping_add(1);
            self.render_test_pattern();
            ctx.request_repaint_after(FRAME_INTERVAL);
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.draw_ui(ui);
    }
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

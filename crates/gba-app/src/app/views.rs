//! Launch/player presentation. Guest advancement and storage polling remain in
//! `App::logic`; view actions enter the existing app predicates and adapters.
use super::*;

const ACCENT: egui::Color32 = egui::Color32::from_rgb(154, 181, 255);
const BACKGROUND: egui::Color32 = egui::Color32::from_rgb(13, 16, 23);
const SURFACE: egui::Color32 = egui::Color32::from_rgb(24, 29, 40);
const RESET_HELP: &str =
    "Restart the game. Unsaved in-game progress may be lost; cartridge saves are retained.";

impl GbaApp {
    /// Keeps diagnostic startup intact while selecting the normal launch/player
    /// layout from installed-session state, including paused and restoring games.
    pub(super) fn draw_ui(&mut self, ui: &mut egui::Ui) {
        let mut owns_keys = self.settings_open;
        if self.debug_ui {
            self.draw_notices(ui);
            self.draw_debug_ui(ui);
        } else {
            ui.painter().rect_filled(ui.max_rect(), 0.0, BACKGROUND);
            if self.loaded {
                owns_keys |= self.draw_player_ui(ui);
            } else {
                owns_keys |= self.draw_launch_ui(ui);
            }
        }
        self.draw_settings(ui.ctx());
        owns_keys |= self.settings_open || ui.ctx().egui_wants_keyboard_input();
        if owns_keys || self.ui_keys_owned {
            self.own_ui_keys(ui.ctx());
        }
        self.ui_keys_owned = owns_keys;
    }

    /// Every exposed click releases previously submitted gameplay keys, even
    /// when the click occurs after this frame's bounded logic step.
    fn action(&mut self, ui: &mut egui::Ui, enabled: bool, label: &str) -> bool {
        let response = ui.add_enabled(
            enabled,
            egui::Button::new(label).min_size(egui::vec2(0.0, 30.0)),
        );
        if response.clicked() {
            self.own_ui_keys(ui.ctx());
            if !self.settings_open {
                response.surrender_focus();
            }
            true
        } else {
            false
        }
    }

    fn open_settings(&mut self, ctx: &egui::Context) {
        self.settings_open = true;
        self.own_ui_keys(ctx);
        self.sync_execution(true, true);
    }

    /// Centered, scrollable startup card. A pending ROM is named explicitly;
    /// successfully installed firmware is never renamed by a failed selection.
    fn draw_launch_ui(&mut self, ui: &mut egui::Ui) -> bool {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(((ui.available_height() - 470.0) * 0.5).max(12.0));
                let width = (ui.available_width() - 24.0).clamp(1.0, 560.0);
                ui.vertical_centered(|ui| {
                    egui::Frame::NONE
                        .fill(SURFACE)
                        .corner_radius(14)
                        .inner_margin(24)
                        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(48, 57, 76)))
                        .show(ui, |ui| {
                            ui.set_width((width - 48.0).max(1.0));
                            ui.vertical_centered(|ui| {
                                ui.label(
                                    egui::RichText::new("Lycan")
                                        .size(32.0)
                                        .strong()
                                        .color(ACCENT),
                                );
                                ui.label(
                                    egui::RichText::new("Game Boy Advance Emulator").size(17.0),
                                );
                                ui.add_space(12.0);
                                ui.label("Load a BIOS and ROM to start playing.");
                                ui.add_space(18.0);
                                let button_width = if ui.available_width() >= 452.0 {
                                    220.0
                                } else {
                                    ui.available_width().min(300.0)
                                };
                                ui.horizontal_wrapped(|ui| {
                                    ui.spacing_mut().item_spacing = egui::vec2(12.0, 12.0);
                                    if ui
                                        .add_enabled(
                                            self.can_pick_bios(),
                                            egui::Button::new("Load BIOS")
                                                .min_size(egui::vec2(button_width, 52.0)),
                                        )
                                        .clicked()
                                    {
                                        self.own_ui_keys(ui.ctx());
                                        self.pick_bios(ui.ctx());
                                    }
                                    if ui
                                        .add_enabled(
                                            self.can_pick_rom(),
                                            egui::Button::new("Load ROM")
                                                .min_size(egui::vec2(button_width, 52.0)),
                                        )
                                        .clicked()
                                    {
                                        self.own_ui_keys(ui.ctx());
                                        self.pick_rom(ui.ctx());
                                    }
                                });
                                ui.add_space(16.0);
                            });
                            self.draw_startup_assets(ui);
                            if let Some(progress) = self.operation().label() {
                                ui.add_space(8.0);
                                ui.label(progress);
                            }
                            if self.pending_rom.is_some()
                                && self.action(ui, true, "Cancel pending ROM")
                            {
                                self.cancel_replacement();
                            }
                            self.draw_notices(ui);
                            ui.add_space(12.0);
                            ui.separator();
                            self.draw_audio_controls(ui, 220.0);
                            ui.add_space(12.0);
                            ui.vertical_centered(|ui| {
                                if self.action(ui, true, "Settings") {
                                    self.open_settings(ui.ctx());
                                }
                                ui.small("Keyboard controls · View all ten bindings in Settings.");
                                ui.small("You can also drop a ROM file here.");
                            });
                        });
                });
            });
        false
    }

    fn draw_startup_assets(&self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.strong("BIOS");
            if self.bios_picker_open {
                ui.label("Reading…");
            } else if let Some(name) = &self.bios_name {
                ui.label("Ready");
                ui.label(name);
            } else {
                ui.label("Missing · supply your 16 KiB BIOS file");
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.strong("ROM");
            if self.picker_open {
                ui.label("Reading…");
            } else if let Some(pending) = &self.pending_rom {
                ui.label("Ready · pending startup");
                ui.label(&pending.name);
            } else {
                ui.label("Missing · select a .gba cartridge");
            }
        });
    }

    /// A bounded toolbar and restrained frame give the remaining area to the
    /// existing nearest-neighbor texture. Long cartridge names get only spare width.
    fn draw_player_ui(&mut self, ui: &mut egui::Ui) -> bool {
        self.sync_texture(ui.ctx());
        let narrow = ui.available_width() < 640.0;
        let menu_open = ui
            .horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(8.0, 6.0);
                ui.label(
                    egui::RichText::new("Lycan")
                        .strong()
                        .size(21.0)
                        .color(ACCENT),
                );
                let menu = ui.menu_button("Game", |ui| self.draw_game_menu(ui));
                let pause_label = if self.user_paused { "Resume" } else { "Pause" };
                if self.action(ui, self.can_pause(), pause_label) {
                    self.toggle_user_pause();
                }
                let reset = ui
                    .add_enabled(
                        self.can_reset(),
                        egui::Button::new("Reset").min_size(egui::vec2(0.0, 30.0)),
                    )
                    .on_hover_text(RESET_HELP);
                if reset.clicked() {
                    self.own_ui_keys(ui.ctx());
                    reset.surrender_focus();
                    self.reset_demo();
                }
                if self.action(ui, true, "Settings") {
                    self.open_settings(ui.ctx());
                }
                let mut audio_open = false;
                if narrow {
                    audio_open = ui
                        .menu_button("Audio", |ui| self.draw_audio_controls(ui, 160.0))
                        .inner
                        .is_some();
                } else {
                    self.draw_audio_controls(ui, 150.0);
                }
                if menu.inner.is_none() && self.ui_keys_owned && !self.settings_open {
                    menu.response.surrender_focus();
                }
                menu.inner.is_some() || audio_open
            })
            .inner;
        // Title has its own bounded row, so file names never displace actions.
        ui.add(egui::Label::new(&self.rom_name).truncate())
            .on_hover_text(&self.rom_name);
        ui.separator();
        self.draw_player_status(ui);
        let available = ui.available_size();
        // Automatic hiding never changes the saved preference, and hidden legends
        // reserve no space. Short views keep the framebuffer as the primary content.
        let deck = available.x >= 600.0 && available.y >= 430.0;
        let show_legend = self.settings.preferences.legend_visible
            && (deck || (available.x >= 400.0 && available.y >= 280.0));
        let legend_height = if show_legend {
            // Include caption and layout spacing so the final row stays inside
            // the player region rather than clipping below the framebuffer.
            (if deck { 112.0 } else { 46.0 })
                + 6.0
                + ui.text_style_height(&egui::TextStyle::Small)
                + 3.0 * ui.spacing().item_spacing.y
        } else {
            0.0
        };
        let screen_area = available - egui::vec2(0.0, legend_height);
        let size = player_screen_size((screen_area - egui::vec2(12.0, 12.0)).max(egui::Vec2::ZERO));
        ui.allocate_ui_with_layout(
            available,
            egui::Layout::top_down(egui::Align::Center),
            |ui| {
                ui.add_space(((screen_area.y - size.y - 12.0) * 0.5).max(0.0));
                egui::Frame::NONE
                    .fill(egui::Color32::BLACK)
                    .inner_margin(6)
                    .corner_radius(6)
                    .show(ui, |ui| {
                        if let Some(texture) = &self.texture
                            && ui
                                .add(
                                    egui::Image::from_texture(texture)
                                        .fit_to_exact_size(size)
                                        .sense(egui::Sense::click()),
                                )
                                .clicked()
                        {
                            ui.ctx().memory_mut(|memory| {
                                if let Some(id) = memory.focused() {
                                    memory.surrender_focus(id);
                                }
                            });
                        }
                    });
                if legend_height > 0.0 {
                    self.draw_controller_legend(ui, deck);
                }
            },
        );
        menu_open
    }

    /// A bounded painted keyboard legend, never a pointer/gameplay controller.
    /// Highlights read the session's submitted logical state, including releases.
    fn draw_controller_legend(&self, ui: &mut egui::Ui, deck: bool) {
        ui.add_space(6.0);
        let width = ui.available_width().min(if deck { 660.0 } else { 600.0 });
        let height = if deck { 112.0 } else { 46.0 };
        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
        if deck {
            ui.painter().rect_filled(rect.shrink(1.0), 20.0, SURFACE);
        }
        // Application slot positions describe the deck; they never change the
        // hardware Button ordering or the preference mapping.
        let positions = [
            [0.87, 0.47],
            [0.74, 0.68],
            [0.17, 0.14],
            [0.83, 0.14],
            [0.55, 0.80],
            [0.39, 0.80],
            [0.17, 0.39],
            [0.17, 0.83],
            [0.06, 0.61],
            [0.28, 0.61],
        ];
        for (slot, button) in BINDING_BUTTONS.into_iter().enumerate() {
            let cell = if deck {
                let [x, y] = positions[slot];
                egui::Rect::from_center_size(
                    rect.min + egui::vec2(width * x, height * y),
                    egui::vec2(if slot == 5 { 98.0 } else { 72.0 }, 22.0),
                )
            } else {
                let column = slot % 5;
                let row = slot / 5;
                egui::Rect::from_min_size(
                    rect.min + egui::vec2(column as f32 * width / 5.0, row as f32 * 23.0),
                    egui::vec2(width / 5.0 - 4.0, 21.0),
                )
            };
            let pressed = self.session.button_pressed(button);
            let stroke = egui::Stroke::new(
                if pressed { 2.5 } else { 1.0 },
                if pressed {
                    ACCENT
                } else {
                    egui::Color32::from_gray(90)
                },
            );
            ui.painter().rect(
                cell,
                4.0,
                if pressed {
                    egui::Color32::from_rgb(45, 61, 91)
                } else {
                    BACKGROUND
                },
                stroke,
                egui::StrokeKind::Inside,
            );
            ui.painter().text(
                cell.center(),
                egui::Align2::CENTER_CENTER,
                &self.binding_labels[slot].legend,
                egui::FontId::proportional(11.0),
                egui::Color32::WHITE,
            );
            ui.interact(
                cell,
                ui.id().with(("binding-legend", slot)),
                egui::Sense::hover(),
            )
            .on_hover_text(self.binding_labels[slot].full);
        }
        ui.label(egui::RichText::new("Keyboard controls; change bindings in Settings.").small());
    }

    /// Reuses the application actions; opening this menu gates keys without
    /// introducing another machine owner, save route or execution clock.
    fn draw_game_menu(&mut self, ui: &mut egui::Ui) {
        if self.action(ui, self.can_pick_rom(), "Load ROM…") {
            self.pick_rom(ui.ctx());
            ui.close();
        }
        if self.loaded {
            ui.small("Replacing BIOS restarts this game after cartridge saves finish.");
        }
        if self.action(ui, self.can_pick_bios(), "Replace BIOS…") {
            self.pick_bios(ui.ctx());
            ui.close();
        }
        ui.separator();
        if self.action(ui, self.can_import_save(), "Import cartridge save…") {
            self.import_cartridge_save(ui.ctx());
            ui.close();
        }
        if self.action(ui, self.can_export_save(), "Export cartridge save…") {
            self.export_cartridge_save(ui.ctx());
            ui.close();
        }
        if self.storage.failed && self.action(ui, !self.storage.busy, "Retry save storage") {
            self.storage.failed = false;
            ui.ctx().request_repaint();
            ui.close();
        }
        if !self.can_import_save() {
            ui.small("Import requires an idle cartridge save operation.");
        }
        if !self.can_export_save() {
            ui.small("Export requires a detected cartridge save capacity.");
        }
        ui.separator();
        ui.strong("Backup override for next ROM");
        self.draw_backup_override(ui);
    }

    /// Backup detection choices remain request-local and available outside F1.
    fn draw_backup_override(&mut self, ui: &mut egui::Ui) {
        egui::ComboBox::from_id_salt("player-backup-override")
            .selected_text(self.backup_override.map_or("Automatic", |kind| match kind {
                gba_session::BackupType::None => "No save hardware",
                gba_session::BackupType::Eeprom => "EEPROM (detect capacity)",
                gba_session::BackupType::Sram => "SRAM",
                gba_session::BackupType::Flash64 => "Flash 64 KiB",
                gba_session::BackupType::Flash128 => "Flash 128 KiB",
                gba_session::BackupType::Eeprom512 => "EEPROM 512 B",
                gba_session::BackupType::Eeprom8k => "EEPROM 8 KiB",
            }))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.backup_override, None, "Automatic");
                for (name, kind) in [
                    ("No save hardware", gba_session::BackupType::None),
                    ("EEPROM (detect capacity)", gba_session::BackupType::Eeprom),
                    ("SRAM", gba_session::BackupType::Sram),
                    ("Flash 64 KiB", gba_session::BackupType::Flash64),
                    ("Flash 128 KiB", gba_session::BackupType::Flash128),
                    ("EEPROM 512 B", gba_session::BackupType::Eeprom512),
                    ("EEPROM 8 KiB", gba_session::BackupType::Eeprom8k),
                ] {
                    ui.selectable_value(&mut self.backup_override, Some(kind), name);
                }
            });
    }

    /// Save banners use lightweight metadata. Snapshot cloning is confined to an
    /// explicit write/export action in the existing adapter path.
    fn draw_player_status(&mut self, ui: &mut egui::Ui) {
        if let Some(progress) = self.operation().label() {
            ui.label(progress);
        }
        if (self.pending_rom.is_some() || self.pending_bios.is_some())
            && self.action(ui, true, "Cancel replacement")
        {
            self.cancel_replacement();
        }
        if self.execution_faulted {
            ui.strong("Emulation stopped · Reset or load another ROM to recover.");
            if self.notice.is_none() {
                ui.label(&self.status);
            }
        } else if self.user_paused {
            ui.label("Paused");
        }
        if self.save_identity.is_some() {
            let status = if self.storage.failed {
                "Cartridge save needs attention"
            } else if self.restoring_save {
                "Restoring cartridge save"
            } else if self.storage.busy || self.session.persistence_dirty() {
                "Saving cartridge data"
            } else {
                "Cartridge save up to date"
            };
            ui.small(status).on_hover_text(&self.storage.status);
            if self.storage.failed {
                egui::Frame::NONE
                    .fill(SURFACE)
                    .inner_margin(10)
                    .corner_radius(6)
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("save-recovery")
                            .max_height(160.0)
                            .show(ui, |ui| self.draw_save_controls(ui));
                    });
            }
        }
        self.draw_notices(ui);
    }

    /// Error details wrap and scroll; the game remains installed beneath recoverable errors.
    fn draw_notices(&mut self, ui: &mut egui::Ui) {
        if self.notice.is_some() || self.settings.warning.is_some() {
            egui::ScrollArea::vertical()
                .id_salt("player-notices")
                .max_height(120.0)
                .show(ui, |ui| {
                    if let Some(notice) = &self.notice {
                        ui.label(notice);
                    }
                    if let Some(warning) = &self.settings.warning {
                        ui.label(warning);
                    }
                    if self.action(ui, true, "Dismiss notice") {
                        self.notice = None;
                        self.settings.warning = None;
                    }
                });
        }
        if self.settings.writes_blocked() && self.action(ui, true, "Reset saved settings") {
            let old_bindings = self.settings.preferences.bindings;
            self.settings.reset_saved_settings();
            if old_bindings != self.settings.preferences.bindings {
                self.bindings_changed(ui.ctx());
            }
            self.audio.volume = self.settings.preferences.volume;
            self.audio.muted = self.settings.preferences.muted;
        }
    }

    /// Uses current explicit audio startup and gain controls. Automatic startup,
    /// browser readiness and fullscreen gesture integration remain separate work.
    fn draw_audio_controls(&mut self, ui: &mut egui::Ui, width: f32) {
        ui.horizontal_wrapped(|ui| {
            if !self.audio.enabled() || self.audio.error().is_some() {
                let label = if self.audio.error().is_some() {
                    "Retry audio"
                } else {
                    "Enable audio"
                };
                if self.action(ui, true, label) {
                    self.audio.start();
                }
            }
            let mute = ui.checkbox(&mut self.audio.muted, "Mute");
            let mut percent = self.audio.volume * 100.0;
            let slider = ui
                .scope(|ui| {
                    ui.spacing_mut().slider_width = width.min(ui.available_width().max(40.0));
                    ui.add(egui::Slider::new(&mut percent, 0.0..=100.0).suffix("%"))
                })
                .inner;
            if slider.changed() || mute.changed() {
                self.audio.volume = percent / 100.0;
                self.own_ui_keys(ui.ctx());
            }
        });
        if let Some(error) = self.audio.error() {
            ui.label(error);
        }
    }

    /// Settings suspend execution while capture owns logical-key events.
    fn draw_settings(&mut self, ctx: &egui::Context) {
        if !self.settings_open {
            return;
        }
        if self.capture.is_none()
            && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            self.settings_open = false;
            ctx.memory_mut(|memory| {
                if let Some(id) = memory.focused() {
                    memory.surrender_focus(id);
                }
            });
            self.own_ui_keys(ctx);
            self.sync_execution(true, true);
            ctx.request_repaint();
            return;
        }
        let mut open = self.settings_open;
        let mut close_clicked = false;
        egui::Window::new("Settings")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(420.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.heading("Controls");
                    ui.label(
                        "Keyboard keys are logical keys; positions may differ across layouts.",
                    );
                    egui::Grid::new("settings-bindings")
                        .num_columns(3)
                        .spacing([24.0, 5.0])
                        .show(ui, |ui| {
                            for (slot, button) in BINDING_BUTTONS.into_iter().enumerate() {
                                ui.label(button_name(button));
                                ui.label(self.binding_labels[slot].full);
                                if self.action(ui, self.capture.is_none(), "Change") {
                                    self.begin_binding_capture(ctx, slot);
                                }
                                ui.end_row();
                            }
                        });
                    if let Some(capture) = &self.capture {
                        ui.label(format!(
                            "Binding {} — press a supported key. Escape cancels.",
                            button_name(BINDING_BUTTONS[capture.slot])
                        ));
                        if self.action(ui, true, "Cancel binding") {
                            self.capture = None;
                            self.capture_notice = Some("Binding cancelled.".into());
                        }
                    }
                    if let Some(notice) = &self.capture_notice {
                        ui.label(notice);
                    }
                    if self.action(ui, self.capture.is_none(), "Reset bindings") {
                        if self.settings.preferences.bindings != crate::settings::DEFAULT_BINDINGS {
                            self.settings.preferences.bindings = crate::settings::DEFAULT_BINDINGS;
                            self.bindings_changed(ctx);
                        }
                        self.capture_notice = Some("Keyboard bindings reset to defaults.".into());
                    }
                    ui.checkbox(
                        &mut self.settings.preferences.legend_visible,
                        "Show controller legend",
                    );
                    ui.label("Keyboard controls; change bindings in Settings.");
                    ui.add_space(12.0);
                    ui.separator();
                    ui.heading("Audio");
                    self.draw_audio_controls(ui, 250.0);
                    ui.add_space(12.0);
                    ui.separator();
                    ui.heading("Display");
                    ui.label("The game fits the window with sharp nearest-neighbor scaling.");
                    ui.add_space(12.0);
                    if self.action(ui, true, "Close settings") {
                        close_clicked = true;
                    }
                });
            });
        if !open || close_clicked {
            self.capture = None;
            self.settings_open = false;
            ctx.memory_mut(|memory| {
                if let Some(id) = memory.focused() {
                    memory.surrender_focus(id);
                }
            });
            self.own_ui_keys(ctx);
            self.sync_execution(true, true);
            ctx.request_repaint();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Read-only binding labels leave users unable to change controls at all.
    // This checks the missing user action rather than widget construction order.
    #[test]
    fn controls_settings_exposes_remapping_actions() {
        let mut app = GbaApp::create(false);
        app.settings_open = true;
        let ctx = egui::Context::default();
        let mut found = false;
        for _ in 0..3 {
            let mut output =
                ctx.run_ui(egui::RawInput::default(), |ui| app.draw_settings(ui.ctx()));
            output.textures_delta.clear();
            found |= output.shapes.iter().any(|shape| {
                matches!(&shape.shape, egui::Shape::Text(text) if text.galley.text() == "Change")
            });
        }
        assert!(found, "Controls settings has no remapping action");
    }
}

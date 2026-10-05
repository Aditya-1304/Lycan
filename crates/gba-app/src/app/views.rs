//! Launch/player presentation. Guest advancement and storage polling remain in
//! `App::logic`; view actions enter the existing app predicates and adapters.
use super::*;

const ACCENT: egui::Color32 = egui::Color32::from_rgb(154, 181, 255);
const BACKGROUND: egui::Color32 = egui::Color32::from_rgb(13, 16, 23);
const SURFACE: egui::Color32 = egui::Color32::from_rgb(24, 29, 40);
const RESET_HELP: &str =
    "Restart the game. Unsaved in-game progress may be lost; cartridge saves are retained.";

/// Centers the load-button group using its actual bounded width. Narrow cards
/// wrap within one button width instead of anchoring the group to the left edge.
fn centered_launch_buttons<R>(
    ui: &mut egui::Ui,
    width: f32,
    contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    ui.vertical_centered(|ui| {
        ui.allocate_ui_with_layout(
            egui::vec2(width, 52.0),
            egui::Layout::left_to_right(egui::Align::Center).with_main_wrap(true),
            contents,
        )
        .inner
    })
    .inner
}

/// Transient navigation only; section selection is not a persisted preference.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub(super) enum SettingsSection {
    #[default]
    Controls,
    Audio,
    Display,
}

impl SettingsSection {
    fn label(self) -> &'static str {
        match self {
            Self::Controls => "Controls",
            Self::Audio => "Audio",
            Self::Display => "Display",
        }
    }
}

impl GbaApp {
    /// Keeps diagnostic startup intact while selecting the normal launch/player
    /// layout from installed-session state, including paused and restoring games.
    pub(super) fn draw_ui(&mut self, ui: &mut egui::Ui) {
        #[cfg(not(target_arch = "wasm32"))]
        let popup_owned_escape = egui::Popup::is_any_open(ui.ctx());
        #[cfg(target_arch = "wasm32")]
        crate::presentation::begin_frame();
        #[cfg(not(target_arch = "wasm32"))]
        if ui
            .ctx()
            .input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::F11))
        {
            self.toggle_fullscreen(ui.ctx());
        }
        let mut owns_keys = false;
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
        // Capture and modal dismissal consume Escape first. Native fullscreen
        // gets only an otherwise unowned Escape; browser exit remains DOM-owned.
        #[cfg(not(target_arch = "wasm32"))]
        if !popup_owned_escape
            && !self.settings_open
            && self.capture.is_none()
            && !egui::Popup::is_any_open(ui.ctx())
            && self.is_fullscreen(ui.ctx())
            && ui
                .ctx()
                .input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
            self.own_ui_keys(ui.ctx());
        }
        owns_keys |= self.settings_open
            || egui::Popup::is_any_open(ui.ctx())
            || ui.ctx().egui_wants_keyboard_input();
        if owns_keys {
            self.own_ui_keys(ui.ctx());
        }
        self.update_browser_input(
            self.execution_allowed() && !self.pointer_ui_owned && !self.keyboard_ui_owned(ui.ctx()),
        );
    }

    /// Every exposed click releases previously submitted gameplay keys, even
    /// when the click occurs after this frame's bounded logic step.
    fn action(&mut self, ui: &mut egui::Ui, enabled: bool, label: &str) -> bool {
        let response = ui.add_enabled(
            enabled,
            egui::Button::new(label).min_size(egui::vec2(0.0, 30.0)),
        );
        if response.gained_focus() {
            response.scroll_to_me(Some(egui::Align::Center));
        }
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
        self.sync_execution(self.host_focused, self.host_visible);
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
                                ui.label("Load a ROM to start. A default BIOS is included.");
                                ui.add_space(18.0);
                                let button_width = if ui.available_width() >= 452.0 {
                                    220.0
                                } else {
                                    ui.available_width().min(300.0)
                                };
                                centered_launch_buttons(ui, if ui.available_width() >= 452.0 { 452.0 } else { button_width }, |ui| {
                                    ui.spacing_mut().item_spacing = egui::vec2(12.0, 12.0);
                                    if ui
                                        .add_enabled(
                                            self.can_pick_bios(),
                                            egui::Button::new("Change BIOS")
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
                            // Bound the header's child UI to its natural width:
                            // egui positions its text before allocating the header.
                            let title = "Backup override for next ROM";
                            let header_width = (ui.painter().layout_no_wrap(
                                title.into(),
                                egui::TextStyle::Button.resolve(ui.style()),
                                ui.visuals().text_color(),
                            ).size().x + ui.spacing().indent + ui.spacing().button_padding.x)
                                .max(ui.spacing().interact_size.x)
                                .min(ui.available_width());
                            ui.vertical_centered(|ui| {
                                ui.allocate_ui_with_layout(
                                    egui::vec2(header_width, ui.spacing().interact_size.y),
                                    egui::Layout::top_down(egui::Align::Min),
                                    |ui| ui.collapsing(title, |ui| {
                                        self.draw_backup_override(ui);
                                        ui.small("Choose only when automatic detection is unresolved; this choice belongs to the next ROM load.");
                                    }),
                                );
                            });
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
                            self.draw_audio_controls_with_layout(ui, 220.0, !self.settings_open, true, false);
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
        let bios = if self.bios_picker_open {
            "Reading…"
        } else {
            self.bios_name
                .as_deref()
                .unwrap_or("Missing · supply your 16 KiB BIOS file")
        };
        let bios_state = if !self.bios_picker_open && self.bios_name.is_some() {
            "Ready · "
        } else {
            ""
        };
        let bios_text = format!("BIOS · {bios_state}{bios}");
        let rom = if self.picker_open {
            "Reading…"
        } else {
            self.pending_rom
                .as_ref()
                .map_or("Missing · select a .gba cartridge", |pending| {
                    pending.name.as_str()
                })
        };
        let rom_state = if !self.picker_open && self.pending_rom.is_some() {
            "Ready · pending startup · "
        } else {
            ""
        };
        let rom_text = format!("ROM · {rom_state}{rom}");
        let font = egui::TextStyle::Body.resolve(ui.style());
        let text_width = |text: &str| {
            ui.painter()
                .layout_no_wrap(text.into(), font.clone(), ui.visuals().text_color())
                .size()
                .x
        };
        let natural_width =
            text_width(&bios_text) + ui.spacing().item_spacing.x + text_width(&rom_text);
        let width = natural_width.min(ui.available_width());
        // Center the visible text group, rather than two full-width columns
        // whose unequal text lengths leave excess space on the right.
        ui.vertical_centered(|ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(width, ui.text_style_height(&egui::TextStyle::Body)),
                egui::Layout::left_to_right(egui::Align::Min),
                |ui| {
                    if natural_width <= width {
                        ui.add(egui::Label::new(&bios_text).wrap());
                        ui.add(egui::Label::new(&rom_text).wrap());
                    } else {
                        ui.columns(2, |columns| {
                            columns[0].add(egui::Label::new(&bios_text).wrap());
                            columns[1].add(egui::Label::new(&rom_text).wrap());
                        });
                    }
                },
            );
        });
    }

    /// A bounded toolbar and restrained frame give the remaining area to the
    /// existing nearest-neighbor texture. Long cartridge names get only spare width.
    fn draw_player_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let height = ui.available_height();
        egui::ScrollArea::vertical()
            .id_salt("player-overflow")
            .auto_shrink([false, false])
            .show(ui, |ui| self.draw_player_contents(ui, height))
            .inner
    }

    fn draw_player_contents(&mut self, ui: &mut egui::Ui, viewport_height: f32) -> bool {
        let top = ui.cursor().top();
        self.sync_texture(ui.ctx());
        let menu_open = ui
            .horizontal(|ui| {
                ui.add_space(16.0);
                ui.label(
                    egui::RichText::new("Lycan")
                        .strong()
                        .size(25.0)
                        .color(ACCENT),
                );
                // Group the Game menu with branding, using the same button
                // height as player actions while retaining its popup behavior.
                ui.add_space(12.0);
                let (menu_response, menu_inner) = egui::containers::menu::MenuButton::from_button(
                    egui::Button::new("Game").min_size(egui::vec2(0.0, 30.0)),
                )
                .ui(ui, |ui| self.draw_game_menu(ui));
                if menu_inner.is_none() && menu_response.clicked() {
                    menu_response.surrender_focus();
                }
                // Player actions stay anchored to the trailing edge. Wrapped
                // rows remain right-aligned in narrower viewports.
                ui.with_layout(
                    egui::Layout::right_to_left(egui::Align::Center).with_main_wrap(true),
                    |ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(8.0, 6.0);
                        ui.add_space(16.0);
                        self.draw_fullscreen_control(ui, !self.settings_open);
                        if self.action(ui, true, "Settings") {
                            self.open_settings(ui.ctx());
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
                        let pause_label = if self.user_paused { "Resume" } else { "Pause" };
                        if self.action(ui, self.can_pause(), pause_label) {
                            self.toggle_user_pause();
                        }
                    },
                );
                menu_inner.is_some()
            })
            .inner;
        // Place audio beneath the actions, sharing the metadata row when there
        // is sufficient room. Narrow views give each group its own bounded row.
        let narrow = ui.available_width() < 640.0;
        if narrow {
            self.draw_player_metadata(ui);
        }
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), 30.0),
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| {
                ui.add_space(16.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(270.0_f32.min(ui.available_width()), 30.0),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        self.draw_audio_controls_with_layout(
                            ui,
                            150.0,
                            !self.settings_open,
                            false,
                            true,
                        );
                    },
                );
                if !narrow {
                    ui.allocate_ui_with_layout(
                        egui::vec2(ui.available_width(), 30.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| self.draw_player_metadata(ui),
                    );
                }
            },
        );
        ui.separator();
        // Empty status containers must not reserve space above the framebuffer.
        // Keep all progress, recovery actions and notices visible when present.
        if self.operation().label().is_some()
            || self.pending_rom.is_some()
            || self.pending_bios.is_some()
            || self.execution_faulted
            || self.user_paused
            || self.storage.failed
            || self.save_failure.is_some()
            || self.notice.is_some()
            || self.settings.warning.is_some()
            || self.settings.writes_blocked()
        {
            egui::ScrollArea::vertical()
                .id_salt("player-status")
                .max_height((viewport_height * 0.3).clamp(40.0, 180.0))
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    egui::Frame::NONE
                        .inner_margin(egui::Margin {
                            left: 16,
                            right: 16,
                            top: 0,
                            bottom: 0,
                        })
                        .show(ui, |ui| self.draw_player_status(ui));
                });
        }
        let available = egui::vec2(
            ui.available_width(),
            (viewport_height - (ui.cursor().top() - top)).max(0.0),
        );
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
        let layout = player_screen_layout(
            (screen_area - egui::vec2(12.0, 12.0)).max(egui::Vec2::ZERO),
            self.settings.preferences.scaling,
            ui.ctx().pixels_per_point(),
        );
        self.integer_fallback = layout.integer_fallback;
        let size = layout.size;
        if size.x <= 0.0 || size.y <= 0.0 {
            self.game_rect = None;
            return menu_open;
        }
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
                        if let Some(texture) = &self.texture {
                            let (id, rect) = ui.allocate_space(size);
                            let pixels_per_point = ui.ctx().pixels_per_point();
                            let origin = egui::pos2(
                                (rect.min.x * pixels_per_point).round() / pixels_per_point,
                                (rect.min.y * pixels_per_point).round() / pixels_per_point,
                            );
                            let rect = egui::Rect::from_min_size(origin, size);
                            ui.painter().image(
                                texture.id(),
                                rect,
                                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                                egui::Color32::WHITE,
                            );
                            let response = ui.interact(rect, id, egui::Sense::click());
                            self.game_rect = Some(rect);
                            if response.clicked() {
                                ui.ctx().memory_mut(|memory| {
                                    if let Some(id) = memory.focused() {
                                        memory.surrender_focus(id);
                                    }
                                });
                            }
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
        // Shared row centers align shoulders, the directional cross and action
        // keys inside the existing deck bounds. Slots remain presentation-only;
        // hardware Button ordering and persisted bindings are unchanged.
        let positions = [
            [0.89, 44.0 / 112.0],
            [0.75, 70.0 / 112.0],
            [0.22, 16.0 / 112.0],
            [0.84, 16.0 / 112.0],
            [0.66, 98.0 / 112.0],
            [0.49, 98.0 / 112.0],
            [0.22, 44.0 / 112.0],
            [0.22, 98.0 / 112.0],
            [0.08, 70.0 / 112.0],
            [0.36, 70.0 / 112.0],
        ];
        for (slot, button) in BINDING_BUTTONS.into_iter().enumerate() {
            let cell = if deck {
                let [x, y] = positions[slot];
                egui::Rect::from_center_size(
                    rect.min + egui::vec2(width * x, height * y),
                    egui::vec2(if slot == 5 { 104.0 } else { 88.0 }, 24.0),
                )
            } else {
                let column = slot % 5;
                let row = slot / 5;
                egui::Rect::from_min_size(
                    rect.min + egui::vec2(column as f32 * width / 5.0, row as f32 * 23.0),
                    egui::vec2(width / 5.0 - 2.0, 22.0),
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
                egui::FontId::proportional(if deck { 12.0 } else { 11.0 }),
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
        ui.set_max_width((ui.ctx().content_rect().width() - 32.0).clamp(1.0, 380.0));
        ui.add(egui::Label::new(&self.rom_name).wrap());
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
        if self.action(
            ui,
            self.loaded && !self.operation_pending(),
            "Exit game to home",
        ) {
            self.request_home(ui.ctx());
            ui.close();
        }
        ui.separator();
        self.draw_save_controls(ui);
        ui.separator();
        ui.strong("Backup override for next ROM");
        self.draw_backup_override(ui);
    }

    /// Backup detection choices remain request-local and available outside F1.
    /// Nested menus use submenu ownership; a standalone ComboBox would replace
    /// the parent popup's memory slot and disappear before a choice can be made.
    fn draw_backup_override(&mut self, ui: &mut egui::Ui) {
        let selected = self.backup_override.map_or("Automatic", |kind| match kind {
            gba_session::BackupType::None => "No save hardware",
            gba_session::BackupType::Eeprom => "EEPROM (detect capacity)",
            gba_session::BackupType::Sram => "SRAM",
            gba_session::BackupType::Flash64 => "Flash 64 KiB",
            gba_session::BackupType::Flash128 => "Flash 128 KiB",
            gba_session::BackupType::Eeprom512 => "EEPROM 512 B",
            gba_session::BackupType::Eeprom8k => "EEPROM 8 KiB",
        });
        if egui::containers::menu::is_in_menu(ui) {
            ui.menu_button(selected, |ui| self.draw_backup_choices(ui));
        } else {
            egui::ComboBox::from_id_salt("player-backup-override")
                .selected_text(selected)
                .show_ui(ui, |ui| self.draw_backup_choices(ui));
        }
    }

    /// Launch and player menus share the same typed next-load override choices.
    fn draw_backup_choices(&mut self, ui: &mut egui::Ui) {
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
    }

    /// Render cartridge identity and save metadata without initiating persistence.
    fn draw_player_metadata(&mut self, ui: &mut egui::Ui) {
        // Keep cartridge metadata aligned with branding. Save status reads only
        // existing persistence metadata and does not initiate storage work.
        ui.horizontal_wrapped(|ui| {
            ui.add_space(16.0);
            ui.add(egui::Label::new(egui::RichText::new(&self.rom_name).size(15.0)).truncate())
                .on_hover_text(&self.rom_name);

            if self.save_identity.is_some() {
                let status = if self.restoring_save {
                    "Restoring save…"
                } else if self.storage.busy || self.session.persistence_dirty() {
                    "Saving…"
                } else if self
                    .session
                    .save_status()
                    .is_some_and(|status| status.len == 0)
                {
                    "Save capacity unresolved"
                } else {
                    "Saved"
                };
                ui.label(status);
            }
        });
    }

    /// Save banners use lightweight metadata. Snapshot cloning is confined to an
    /// explicit write/export action in the existing adapter path.
    fn draw_player_status(&mut self, ui: &mut egui::Ui) {
        if let Some(progress) = self.operation().label() {
            ui.label(progress);
        }
        if (self.pending_rom.is_some() || self.pending_bios.is_some() || self.home_pending)
            && self.action(
                ui,
                true,
                if self.home_pending {
                    "Cancel return home"
                } else {
                    "Cancel replacement"
                },
            )
        {
            self.cancel_replacement();
        }
        if self.execution_faulted {
            ui.strong("Emulation stopped · Reset or load another ROM to recover.");
            // Export feedback must not hide the stopped guest’s actual reason.
            ui.add(egui::Label::new(&self.status).wrap());
        } else if self.user_paused {
            ui.label("Paused");
        }
        if self.storage.failed || self.save_failure.is_some() {
            egui::Frame::NONE.fill(SURFACE).inner_margin(10).corner_radius(6)
                .show(ui, |ui| {
                    if let Some(failure) = &self.save_failure {
                        ui.strong(failure.kind.label());
                        ui.add(egui::Label::new(&failure.message).wrap());
                    } else {
                        ui.strong("Save storage needs attention");
                        ui.add(egui::Label::new(&self.storage.status).wrap());
                    }
                    ui.label("Closing or refreshing may lose unsaved changes. Export does not release a replacement or close barrier.");
                    self.draw_save_controls(ui);
                });
        }
        self.draw_notices(ui);
    }

    /// One set of recovery actions serves the menu, persistent banner and debug
    /// view. Rendering reads metadata only; snapshots belong to actual exports.
    pub(super) fn draw_save_controls(&mut self, ui: &mut egui::Ui) {
        if self.save_identity.is_none() {
            ui.label("Cartridge save import/export unavailable: no supported backup hardware is selected.");
            ui.small("For unresolved detection, choose a backup override and reload the ROM. No type is inferred from its filename.");
            return;
        }
        if self.restoring_save {
            ui.label("Stored save has not been restored. Retry or import a validated backup; Export cannot recover unvalidated cartridge bytes.");
        } else if self.session.save_status().is_none() {
            ui.label("Clock settings persist automatically; this cartridge has no raw backup to import/export.");
        } else if self
            .session
            .save_status()
            .is_some_and(|status| status.len == 0)
        {
            ui.label("Backup capacity is unresolved. Supported restore/import can resolve EEPROM capacity; Export becomes available afterward.");
            ui.small("You can also select an explicit capacity under Backup override for next ROM, then reload through the existing save barrier.");
        }
        ui.horizontal_wrapped(|ui| {
            if self.storage.failed && self.action(ui, !self.storage.busy, "Retry save storage") {
                self.storage.failed = false;
                ui.ctx().request_repaint();
            }
            if self.action(ui, self.can_import_save(), "Import cartridge save…") {
                self.import_cartridge_save(ui.ctx());
            }
            if self.action(ui, self.can_export_save(), "Export cartridge save…") {
                self.export_cartridge_save(ui.ctx());
            }
        });
        if self.storage.busy {
            ui.label("Save operation in progress; import/export wait for its completion.");
        } else if self.pending_rom.is_some()
            || self.pending_bios.is_some()
            || self.home_pending
            || self.close_pending()
        {
            ui.small("Import waits until replacement, return home or close is cancelled. Export leaves the save barrier intact.");
        }
        ui.small("Import validates format, ROM identity and capacity, then restarts paused. Resume explicitly when ready.");
        ui.small(
            "Export prepares a file/download; it does not acknowledge a failed automatic write.",
        );
    }

    /// Error details wrap and scroll; the game remains installed beneath recoverable errors.
    fn draw_notices(&mut self, ui: &mut egui::Ui) {
        if self.notice.is_some() || self.settings.warning.is_some() {
            egui::ScrollArea::vertical()
                .id_salt("player-notices")
                .max_height(120.0)
                .show(ui, |ui| {
                    if let Some(notice) = &self.notice {
                        ui.strong(notice.kind.label());
                        ui.add(egui::Label::new(&notice.message).wrap());
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
                self.bindings_changed(ui.ctx(), old_bindings);
            }
            self.audio.volume = self.settings.preferences.volume;
            self.audio.muted = self.settings.preferences.muted;
            self.audio.apply_gain();
        }
    }

    /// Actual platform state drives the label, including browser Escape and
    /// request rejection. The compact toolbar remains in the normal layout.
    fn is_fullscreen(&self, ctx: &egui::Context) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            ctx.input(|input| input.viewport().fullscreen.unwrap_or(false))
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = ctx;
            crate::presentation::fullscreen()
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn toggle_fullscreen(&mut self, ctx: &egui::Context) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!self.is_fullscreen(ctx)));
        self.own_ui_keys(ctx);
    }

    fn draw_fullscreen_control(&mut self, ui: &mut egui::Ui, enabled: bool) {
        let label = if self.is_fullscreen(ui.ctx()) {
            "Exit fullscreen"
        } else {
            "Fullscreen"
        };
        let response = ui.add_enabled(
            enabled,
            egui::Button::new(label).min_size(egui::vec2(0.0, 30.0)),
        );
        if response.gained_focus() {
            response.scroll_to_me(Some(egui::Align::Center));
        }
        #[cfg(target_arch = "wasm32")]
        if enabled {
            crate::presentation::register("fullscreen", &response, ui.clip_rect());
        }
        if response.clicked() {
            self.own_ui_keys(ui.ctx());
            response.surrender_focus();
            #[cfg(not(target_arch = "wasm32"))]
            self.toggle_fullscreen(ui.ctx());
        }
        #[cfg(target_arch = "wasm32")]
        {
            let error = crate::presentation::error();
            if !error.is_empty() {
                ui.label(format!("Fullscreen unavailable: {error}"));
            }
        }
    }

    /// All controls edit the same remembered volume and independent mute. Device
    /// readiness is asynchronous on web; diagnostics stay in the debug view.
    pub(super) fn draw_audio_controls(&mut self, ui: &mut egui::Ui, width: f32, enabled: bool) {
        self.draw_audio_controls_with_layout(ui, width, enabled, false, false);
    }

    /// Startup centers the controls; the player omits redundant labels while
    /// retaining actionable audio states. Settings keeps its descriptive labels.
    fn draw_audio_controls_with_layout(
        &mut self,
        ui: &mut egui::Ui,
        width: f32,
        enabled: bool,
        centered: bool,
        compact_labels: bool,
    ) {
        use crate::audio::AudioState;
        let state = self.audio.state();
        ui.add_enabled_ui(enabled, |ui| {
            let controls = |ui: &mut egui::Ui| {
                if matches!(
                    state,
                    AudioState::NeedsInteraction | AudioState::Unavailable
                ) {
                    let label = if state == AudioState::Unavailable {
                        "Retry audio"
                    } else {
                        "Enable audio"
                    };
                    let response = ui.add(egui::Button::new(label).min_size(egui::vec2(0.0, 30.0)));
                    #[cfg(target_arch = "wasm32")]
                    if enabled {
                        crate::presentation::register("audio", &response, ui.clip_rect());
                    }
                    if response.clicked() {
                        self.own_ui_keys(ui.ctx());
                        response.surrender_focus();
                        #[cfg(not(target_arch = "wasm32"))]
                        self.audio.start();
                    }
                }
                let mute = ui
                    .checkbox(&mut self.audio.muted, "Mute")
                    .on_hover_text("Mute the speaker without changing remembered volume");
                let mut percent = self.audio.volume * 100.0;
                let slider = ui
                    .scope(|ui| {
                        let slider_width = width.min(ui.available_width().max(40.0));
                        ui.spacing_mut().slider_width = slider_width;
                        let widget = egui::Slider::new(&mut percent, 0.0..=100.0)
                            .suffix("%")
                            .text(if centered || compact_labels {
                                ""
                            } else {
                                "Volume"
                            })
                            .show_value(!centered);
                        if centered {
                            // Slider creates its own horizontal UI. Bound that UI
                            // to the track width so the centered parent positions
                            // the track itself, rather than a full-width wrapper.
                            ui.allocate_ui_with_layout(
                                egui::vec2(slider_width, ui.spacing().interact_size.y),
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| ui.add(widget),
                            )
                            .inner
                        } else {
                            ui.add(widget)
                        }
                    })
                    .inner;
                let value_changed = centered
                    && ui
                        .add(
                            egui::DragValue::new(&mut percent)
                                .range(0.0..=100.0)
                                .suffix("%")
                                .fixed_decimals(1),
                        )
                        .changed();
                if slider.changed() || mute.changed() || value_changed {
                    self.audio.volume = percent / 100.0;
                    self.audio.apply_gain();
                    self.own_ui_keys(ui.ctx());
                }
                if (!centered && !compact_labels) || state != AudioState::Ready {
                    ui.label(match state {
                        #[cfg(target_arch = "wasm32")]
                        AudioState::Starting => "Starting",
                        AudioState::Ready if self.audio.muted => "Muted",
                        AudioState::Ready => "Ready",
                        AudioState::NeedsInteraction => "Needs interaction",
                        AudioState::Unavailable => "Unavailable",
                    });
                }
            };
            if centered {
                ui.vertical_centered(controls);
            } else {
                ui.horizontal_wrapped(controls);
            }
            if let Some(error) = self.audio.error() {
                ui.label(error);
            }
        });
    }

    /// Section changes stay within the same input-owning settings dialog. Cancel
    /// capture before hiding Controls so later keys cannot mutate an unseen binding.
    fn select_settings_section(&mut self, ctx: &egui::Context, section: SettingsSection) {
        if self.settings_section != section {
            let active_capture = self.capture.take().is_some();
            let pending_capture = self.pending_capture.take().is_some();
            if active_capture || pending_capture {
                self.capture_notice = Some("Binding cancelled.".into());
            }
            self.settings_section = section;
            self.own_ui_keys(ctx);
            ctx.request_repaint();
        }
    }

    fn draw_controls_settings(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.heading("Controls");
        ui.label("Keyboard keys are logical keys; positions may differ across layouts.");
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
                let previous = self.settings.preferences.bindings;
                self.settings.preferences.bindings = crate::settings::DEFAULT_BINDINGS;
                self.bindings_changed(ctx, previous);
            }
            self.capture_notice = Some("Keyboard bindings reset to defaults.".into());
        }
        ui.checkbox(
            &mut self.settings.preferences.legend_visible,
            "Show controller legend",
        );
        ui.label("Keyboard controls; change bindings in Settings.");
    }

    fn draw_audio_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading("Audio");
        self.draw_audio_controls(ui, 250.0, true);
    }

    fn draw_display_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading("Display");
        ui.horizontal_wrapped(|ui| {
            ui.selectable_value(
                &mut self.settings.preferences.scaling,
                crate::settings::Scaling::Fit,
                "Fit window",
            );
            ui.selectable_value(
                &mut self.settings.preferences.scaling,
                crate::settings::Scaling::Integer,
                "Integer pixels",
            );
        });
        ui.label("Integer uses whole physical pixels per GBA pixel; Fit fills the available 3:2 area. Both use nearest-neighbor filtering.");
        if self.settings.preferences.scaling == crate::settings::Scaling::Integer
            && self.integer_fallback
        {
            ui.label("Fit used because the window is too small for 1×.");
        }
        self.draw_fullscreen_control(ui, true);
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
            self.sync_execution(self.host_focused, self.host_visible);
            ctx.request_repaint();
            return;
        }
        let mut open = self.settings_open;
        let mut close_clicked = false;
        egui::Window::new("Settings")
            // Keep old stacked-layout geometry from overriding the larger default.
            .id(egui::Id::new("settings-sections"))
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_size(egui::vec2(660.0, 500.0))
            .min_size(egui::vec2(320.0, 240.0))
            .show(ctx, |ui| {
                let footer_height = 30.0 + 2.0 * ui.spacing().item_spacing.y + 6.0;
                let body_height = (ui.available_height() - footer_height).max(0.0);
                let sidebar_width = (ui.available_width() * 0.22).clamp(80.0, 140.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), body_height),
                    egui::Layout::left_to_right(egui::Align::Min),
                    |ui| {
                        ui.allocate_ui_with_layout(
                            egui::vec2(sidebar_width, body_height),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                for section in [
                                    SettingsSection::Controls,
                                    SettingsSection::Audio,
                                    SettingsSection::Display,
                                ] {
                                    let response = ui.add_sized(
                                        [sidebar_width, 32.0],
                                        egui::Button::new(section.label())
                                            .selected(self.settings_section == section),
                                    );
                                    if response.clicked() {
                                        self.select_settings_section(ctx, section);
                                    }
                                }
                            },
                        );
                        ui.separator();
                        ui.allocate_ui_with_layout(
                            egui::vec2(ui.available_width(), body_height),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                egui::ScrollArea::vertical()
                                    .id_salt(("settings-section", self.settings_section))
                                    .max_height(body_height)
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| match self.settings_section {
                                        SettingsSection::Controls => {
                                            self.draw_controls_settings(ui, ctx)
                                        }
                                        SettingsSection::Audio => self.draw_audio_settings(ui),
                                        SettingsSection::Display => self.draw_display_settings(ui),
                                    });
                            },
                        );
                    },
                );
                ui.separator();
                // The footer sits outside section scrolling, keeping Close visible.
                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), 30.0),
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        close_clicked = self.action(ui, true, "Close");
                    },
                );
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
            self.sync_execution(self.host_focused, self.host_visible);
            ctx.request_repaint();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Existing popup tests cover Escape, but not a nested selector being closed
    // by its parent before the user can choose a backup type.
    #[test]
    fn game_menu_backup_selector_accepts_a_choice() {
        fn frame(
            app: &mut GbaApp,
            ctx: &egui::Context,
            events: Vec<egui::Event>,
        ) -> egui::FullOutput {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1000.0, 700.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| app.draw_ui(ui),
            );
            output.textures_delta.clear();
            output
        }
        fn click(
            app: &mut GbaApp,
            ctx: &egui::Context,
            output: &mut egui::FullOutput,
            label: &str,
        ) {
            let pos = output
                .shapes
                .iter()
                .find_map(|shape| {
                    if let egui::Shape::Text(text) = &shape.shape
                        && text.galley.text() == label
                    {
                        Some(text.pos + text.galley.size() * 0.5)
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| {
                    panic!(
                        "{label} must be visible; rendered: {:?}",
                        output
                            .shapes
                            .iter()
                            .filter_map(|shape| if let egui::Shape::Text(text) = &shape.shape {
                                Some(text.galley.text())
                            } else {
                                None
                            })
                            .collect::<Vec<_>>()
                    )
                });
            for pressed in [true, false] {
                *output = frame(
                    app,
                    ctx,
                    vec![
                        egui::Event::PointerMoved(pos),
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                );
            }
            *output = frame(app, ctx, vec![]);
        }
        let mut app = GbaApp::create(false);
        app.load_rom_bytes("buttons.gba", BUTTONS_ROM);
        let ctx = egui::Context::default();
        let mut output = frame(&mut app, &ctx, vec![]);
        click(&mut app, &ctx, &mut output, "Game");
        click(&mut app, &ctx, &mut output, "Automatic");
        click(&mut app, &ctx, &mut output, "SRAM");
        assert_eq!(app.backup_override, Some(gba_session::BackupType::Sram));
    }

    // Changing sections must not leave an invisible binding capture active.
    // Existing close/settings tests do not cover navigation inside the dialog.
    #[test]
    fn changing_settings_section_cancels_capture_without_resuming_gameplay() {
        let mut app = GbaApp::create(false);
        app.load_rom_bytes("buttons.gba", BUTTONS_ROM);
        let ctx = egui::Context::default();
        app.open_settings(&ctx);
        app.begin_binding_capture(&ctx, 0);
        assert!(app.capture.is_some());
        app.select_settings_section(&ctx, SettingsSection::Audio);
        assert!(app.capture.is_none());
        assert!(app.pending_capture.is_none());
        assert!(app.settings_open);
        assert!(!app.execution_allowed());
        assert_eq!(app.settings.preferences.bindings[0], egui::Key::Z);
    }

    // Catches one Escape dismissing the Game menu and exiting fullscreen at once.
    // Existing capture/settings tests do not exercise egui popup dismissal.
    #[test]
    fn escape_dismisses_game_menu_before_native_fullscreen() {
        let mut app = GbaApp::create(false);
        app.loaded = true;
        let ctx = egui::Context::default();
        let raw = |events| {
            let mut input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1000.0, 700.0),
                )),
                events,
                ..Default::default()
            };
            input
                .viewports
                .get_mut(&egui::ViewportId::ROOT)
                .unwrap()
                .fullscreen = Some(true);
            input
        };
        let mut output = ctx.run_ui(raw(vec![]), |ui| app.draw_ui(ui));
        output.textures_delta.clear();
        let game = output
            .shapes
            .iter()
            .find_map(|shape| {
                if let egui::Shape::Text(text) = &shape.shape
                    && text.galley.text() == "Game"
                {
                    Some(text.pos + text.galley.size() * 0.5)
                } else {
                    None
                }
            })
            .expect("Game menu is visible");
        for pressed in [true, false] {
            let mut output = ctx.run_ui(
                raw(vec![
                    egui::Event::PointerMoved(game),
                    egui::Event::PointerButton {
                        pos: game,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ]),
                |ui| app.draw_ui(ui),
            );
            output.textures_delta.clear();
        }
        assert!(egui::Popup::is_any_open(&ctx));
        let mut output = ctx.run_ui(
            raw(vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }]),
            |ui| app.draw_ui(ui),
        );
        output.textures_delta.clear();
        assert!(!egui::Popup::is_any_open(&ctx));
        assert!(
            !output
                .viewport_output
                .values()
                .flat_map(|viewport| &viewport.commands)
                .any(|command| matches!(command, egui::ViewportCommand::Fullscreen(false)))
        );
    }

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

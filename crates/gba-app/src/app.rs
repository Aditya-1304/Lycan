mod views;

use crate::audio::Audio;
use eframe::egui;
use gba_session::{BUTTONS, Button, CYCLES_PER_FRAME, Cycle, SCREEN_HEIGHT, SCREEN_WIDTH, Session};
use sha2::{Digest, Sha256};
use std::{sync::Arc, time::Duration};
use web_time::Instant;

/// Completion of a picker or dropped-file request. None means dialog cancellation.
struct RomRead {
    generation: u64,
    backup_override: Option<gba_session::BackupType>,
    result: Option<(String, Result<Vec<u8>, String>)>,
}

/// Firmware completions share the request generation with ROM selection so a
/// newer drop cannot be restarted by an older picker completion.
struct BiosRead {
    generation: u64,
    result: Option<(String, Result<Vec<u8>, String>)>,
}

/// Validated firmware retained until the installed cartridge's save barrier clears.
struct PendingBios {
    name: String,
    bytes: Vec<u8>,
}

/// ROM bytes waiting for startup firmware or the preceding session's save barrier.
struct PendingRom {
    name: String,
    bytes: Vec<u8>,
    backup_override: Option<gba_session::BackupType>,
    /// Startup waits for firmware; replacement waits only for the existing save barrier.
    requires_bios: bool,
}

/// Presentation progress is derived from bounded operation facts, never status text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Operation {
    Idle,
    ReadingBios,
    ReadingRom,
    AwaitingBios,
    RestoringSave,
    ImportingSave,
    ReplacingRom,
    ReplacingBios,
    Closing,
}

impl Operation {
    fn label(self) -> Option<&'static str> {
        match self {
            Self::Idle => None,
            Self::ReadingBios => Some("Selecting/reading BIOS…"),
            Self::ReadingRom => Some("Selecting/reading ROM…"),
            Self::AwaitingBios => Some("ROM ready; Load BIOS to start."),
            Self::RestoringSave => Some("Restoring save…"),
            Self::ImportingSave => Some("Importing save…"),
            Self::ReplacingRom => Some("Saving current game before loading another ROM."),
            Self::ReplacingBios => Some("Saving current game before replacing BIOS…"),
            Self::Closing => Some("Saving current game before closing…"),
        }
    }
}

/// Operation ownership keeps errors beside the relevant controls without parsing
/// human-readable messages to select behavior or recovery actions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NoticeKind {
    Bios,
    Rom,
    SaveRestore,
    SaveWrite,
    SaveImport,
    SaveExport,
    Emulation,
}
impl NoticeKind {
    fn label(self) -> &'static str {
        match self {
            Self::Bios => "BIOS loading",
            Self::Rom => "ROM loading",
            Self::SaveRestore => "Save restoration failed",
            Self::SaveWrite => "Automatic save failed",
            Self::SaveImport => "Save import",
            Self::SaveExport => "Save export",
            Self::Emulation => "Emulation stopped",
        }
    }
}

/// Bounded feedback: one operation notice and one persistent storage failure.
/// Export/import feedback cannot erase a failed automatic-write recovery banner.
struct AppNotice {
    kind: NoticeKind,
    message: String,
}

/// Original score cartridges use the normal loader and persistence route.
const FLASH_DIAGNOSTIC_ROM: &[u8] =
    include_bytes!("../../../fixtures/diagnostics/gba-tests/save/flash64.gba");
const BANKED_DIAGNOSTIC_ROM: &[u8] =
    include_bytes!("../../../fixtures/diagnostics/gba-tests/save/flash128.gba");
const EEPROM512_ROM: &[u8] = include_bytes!("../../../fixtures/diagnostics/eeprom512-score.gba");
const EEPROM8K_ROM: &[u8] = include_bytes!("../../../fixtures/diagnostics/eeprom8k-score.gba");
const BANKED_ROM: &[u8] = include_bytes!("../../../fixtures/diagnostics/flash-banked-score.gba");
const FLASH_ROM: &[u8] = include_bytes!("../../../fixtures/diagnostics/flash-score.gba");
const RTC_ROM: &[u8] = include_bytes!("../../../fixtures/diagnostics/rtc.gba");
const SRAM_ROM: &[u8] = include_bytes!("../../../fixtures/diagnostics/sram.gba");

/// Original display diagnostics use controlled ARM startup; retail ROMs retain BIOS boot.
const DISPLAY_DIAGNOSTIC_ROMS: [&[u8]; 9] = [
    include_bytes!("../../../fixtures/diagnostics/raster.gba"),
    include_bytes!("../../../fixtures/diagnostics/blend.gba"),
    include_bytes!("../../../fixtures/diagnostics/window.gba"),
    include_bytes!("../../../fixtures/diagnostics/affine-object.gba"),
    include_bytes!("../../../fixtures/diagnostics/affine-mode-1.gba"),
    include_bytes!("../../../fixtures/diagnostics/affine-mode-2.gba"),
    include_bytes!("../../../fixtures/diagnostics/affine-mode-3.gba"),
    include_bytes!("../../../fixtures/diagnostics/affine-mode-4.gba"),
    include_bytes!("../../../fixtures/diagnostics/affine-mode-5.gba"),
];

const WIDTH: usize = SCREEN_WIDTH;
const HEIGHT: usize = SCREEN_HEIGHT;

const BUTTONS_ROM: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/diagnostics/buttons.gba"
));

/// Loading this artifact uses the same session,
/// master-clock pacing, and completed-frame presentation as dropped ROMs.
const PALETTE_ROM: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/diagnostics/palette.gba"
));

/// Guest calculations use the shared session and timed display route on both
/// platforms; the application only observes the guest's completion mailbox.
const CALCULATIONS_ROM: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/diagnostics/calculations.gba"
));

/// The bitmap-copy guest uses the shared loader and frame pacing on native and
/// browser hosts; copying, redraw, and return all execute inside the emulated CPU.
const COPY_ROM: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/diagnostics/copy.gba"
));

/// ARM draws the bar after a Thumb routine updates its guest-owned counter.
const COUNTER_ROM: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/diagnostics/counter.gba"
));

/// Pinned upstream diagnostic bytes also identify the explicit test-firmware
/// loading route, including when the same ROM is opened from a file dialog.
const ARM_DIAGNOSTIC_ROM: &[u8] =
    include_bytes!("../../../fixtures/diagnostics/gba-tests/arm/arm.gba");
const THUMB_DIAGNOSTIC_ROM: &[u8] =
    include_bytes!("../../../fixtures/diagnostics/gba-tests/thumb/thumb.gba");

/// Original mode-0 scene with guest-owned scrolling and the pinned reference ROM.
const TILED_ROM: &[u8] = include_bytes!("../../../fixtures/diagnostics/tiled.gba");
const STRIPES_ROM: &[u8] = include_bytes!("../../../fixtures/diagnostics/gba-tests/stripes.gba");
const TILED_INPUT: &[(Cycle, Button, bool)] =
    &include!("../../../fixtures/diagnostics/tiled/input.rs");

/// Explicit original IRQ diagnostic; normal cartridges never map test firmware.
const KEYPAD_OR_ROM: &[u8] = include_bytes!("../../../fixtures/diagnostics/keypad-or.gba");
const KEYPAD_AND_ROM: &[u8] = include_bytes!("../../../fixtures/diagnostics/keypad-and.gba");
/// The same logical input deadlines are used by the bounded headless runner.
const KEYPAD_INPUT: &[(Cycle, Button, bool)] =
    &include!("../../../fixtures/diagnostics/keypad/input.rs");
const VBLANK_ROM: &[u8] = include_bytes!("../../../fixtures/diagnostics/vblank.gba");
/// Original paired noise events with all PSG voices and both Direct Sound FIFOs.
const NOISE_ROM: &[u8] = include_bytes!("../../../fixtures/diagnostics/noise.gba");
/// Original wave effect scene mixed with pulse voices and Direct Sound.
const WAVE_ROM: &[u8] = include_bytes!("../../../fixtures/diagnostics/wave.gba");
/// Original two-voice pulse scene, using the same explicit firmware as PCM.
const PULSE_ROM: &[u8] = include_bytes!("../../../fixtures/diagnostics/pulse.gba");
const PCM_ROM: &[u8] = include_bytes!("../../../fixtures/diagnostics/pcm.gba");
const PCM_INPUT: &[(Cycle, Button, bool)] = &include!("../../../fixtures/diagnostics/pcm/input.rs");

const DMA_ROM: &[u8] = include_bytes!("../../../fixtures/diagnostics/dma.gba");
const DMA_INPUT: &[(Cycle, Button, bool)] = &include!("../../../fixtures/diagnostics/dma/input.rs");
/// IRQ variant shares the polling scene assets and scripted logical input.
const IRQ_SPRITES_ROM: &[u8] = include_bytes!("../../../fixtures/diagnostics/irq-sprites.gba");
/// Original guest-owned sprite scene and its independently verified replay.
const SPRITES_ROM: &[u8] = include_bytes!("../../../fixtures/diagnostics/sprites.gba");
const SPRITE_INPUT: &[(Cycle, Button, bool)] =
    &include!("../../../fixtures/diagnostics/sprites/input.rs");
const MEMORY_ROM: &[u8] =
    include_bytes!("../../../fixtures/diagnostics/gba-tests/memory/memory.gba");

/// The replay uses the same ordered cycle transitions verified by gba-tools.
const DEMO_INPUT: &[(Cycle, Button, bool)] = &include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/diagnostics/buttons/input.rs"
));

use crate::input::{
    BINDING_BUTTONS, BindingLabel, Capture, CaptureResult, InputOwner, binding_labels, button_name,
};

/// Shared native/browser application displaying pixels produced by guest execution.
pub struct GbaApp {
    /// Presentation mode only; switching modes preserves the active session.
    debug_ui: bool,
    settings: crate::settings::Settings,
    /// A settings window temporarily owns input without changing explicit pause.
    settings_open: bool,
    settings_section: views::SettingsSection,
    /// Transient capture state never reaches persisted preferences.
    capture: Option<Capture>,
    /// One deferred result transfers capture across egui's begin-pass boundary.
    /// No event vector is cloned, retained or queued by the application.
    pending_capture: Option<(CaptureResult, bool)>,
    /// Capture-only held keys preserve repeat detection when its presses are
    /// filtered before egui. The fixed enum domain requires no growing collection.
    capture_keys_down: [bool; egui::Key::ALL.len()],
    capture_notice: Option<String>,
    /// Formatting is cached at load/edit boundaries, not during gameplay drawing.
    binding_labels: [BindingLabel; 10],
    /// Still-held keys remain ineligible after a UI interaction until released.
    suppressed_keys: [bool; 10],
    /// Actual key events retain held state when egui clears focus-local keys.
    /// A missed background key-up is handled conservatively: release once on return.
    host_keys_down: [bool; 10],
    input_owner: InputOwner,
    /// Host facts are updated before completions/actions; no action assumes focus.
    host_focused: bool,
    host_visible: bool,
    /// The last rendered game surface distinguishes gameplay clicks from controls
    /// before drawing processes this frame's pointer activation.
    game_rect: Option<egui::Rect>,
    pointer_ui_owned: bool,
    #[cfg(target_arch = "wasm32")]
    browser_revision: u32,
    #[cfg(target_arch = "wasm32")]
    browser_gameplay: bool,
    notice: Option<AppNotice>,
    save_failure: Option<AppNotice>,
    integer_fallback: bool,
    /// User intent survives file operations, restoration, focus loss and reset.
    user_paused: bool,
    /// Faults are independent of user pause and clear only after reset/install.
    execution_faulted: bool,
    session: Session,
    bios_hash: Option<String>,
    bios_name: Option<String>,
    pending_bios: Option<PendingBios>,
    bios_picker_open: bool,
    bios_sender: std::sync::mpsc::Sender<BiosRead>,
    bios_receiver: std::sync::mpsc::Receiver<BiosRead>,
    storage: crate::saves::Storage,
    save_identity: Option<crate::saves::Identity>,
    save_generation: u64,
    restoring_save: bool,
    save_import_open: bool,
    #[cfg(not(target_arch = "wasm32"))]
    close_when_saved: bool,
    pending_rom: Option<PendingRom>,

    audio: Audio,
    rom_name: String,
    // Typed choices prevent invalid overrides; each choice applies to the next load.
    backup_override: Option<gba_session::BackupType>,
    loaded: bool,
    host_origin: Instant,
    replay_deadline: Option<Cycle>,
    last_guest_generation: u64,
    core_times: Measurements,
    conversion_times: Measurements,
    upload_times: Measurements,
    screen_image: Arc<egui::ColorImage>,
    texture: Option<egui::TextureHandle>,
    image_generation: u64,
    uploaded_generation: Option<u64>,
    status: String,
    // Optional development capture uses eframe's actual rendered native viewport.
    #[cfg(not(target_arch = "wasm32"))]
    capture_path: Option<std::path::PathBuf>,
    #[cfg(not(target_arch = "wasm32"))]
    capture_requested: bool,
    // File work completes off the UI loop. Generations prevent stale reads from
    // replacing a newer dropped file or built-in scene on either platform.
    rom_sender: std::sync::mpsc::Sender<RomRead>,
    rom_receiver: std::sync::mpsc::Receiver<RomRead>,
    load_generation: u64,
    picker_open: bool,
}

impl GbaApp {
    /// Starts a clean player session, or loads the PCM fixture for diagnostics.
    pub fn new(cc: &eframe::CreationContext<'_>, debug_ui: bool) -> Self {
        let mut app =
            Self::create_with_settings(debug_ui, crate::settings::Settings::load(cc.storage));
        // Device startup is excluded from headless constructors. Saved gain/mute
        // are installed before a native stream can play or a web gesture unlocks.
        app.audio.initialize();
        #[cfg(target_arch = "wasm32")]
        crate::presentation::initialize(
            &cc.egui_ctx,
            if app.audio.muted {
                0.0
            } else {
                app.audio.volume
            },
        );
        app
    }

    #[cfg(test)]
    fn create(debug_ui: bool) -> Self {
        Self::create_with_settings(debug_ui, crate::settings::Settings::default())
    }

    fn create_with_settings(debug_ui: bool, settings: crate::settings::Settings) -> Self {
        let mut audio = Audio::default();
        audio.volume = settings.preferences.volume;
        audio.muted = settings.preferences.muted;
        let (rom_sender, rom_receiver) = std::sync::mpsc::channel();
        let (bios_sender, bios_receiver) = std::sync::mpsc::channel();
        let mut app = Self {
            debug_ui,
            binding_labels: binding_labels(&settings.preferences.bindings),
            settings,
            settings_open: false,
            settings_section: views::SettingsSection::default(),
            capture: None,
            pending_capture: None,
            capture_keys_down: [false; egui::Key::ALL.len()],
            capture_notice: None,
            suppressed_keys: [false; 10],
            host_keys_down: [false; 10],
            input_owner: InputOwner::Ui,
            host_focused: true,
            host_visible: true,
            game_rect: None,
            pointer_ui_owned: false,
            #[cfg(target_arch = "wasm32")]
            browser_revision: 0,
            #[cfg(target_arch = "wasm32")]
            browser_gameplay: false,
            notice: None,
            save_failure: None,
            integer_fallback: false,
            user_paused: false,
            execution_faulted: false,
            session: Session::new(),
            bios_hash: None,
            bios_name: None,
            pending_bios: None,
            bios_picker_open: false,
            bios_sender,
            bios_receiver,
            storage: crate::saves::Storage::default(),
            save_identity: None,
            save_generation: 0,
            restoring_save: false,
            save_import_open: false,
            #[cfg(not(target_arch = "wasm32"))]
            close_when_saved: false,
            pending_rom: None,

            audio,
            rom_name: String::new(),
            backup_override: None,
            loaded: false,
            host_origin: Instant::now(),
            replay_deadline: None,
            last_guest_generation: 0,
            core_times: Measurements::default(),
            conversion_times: Measurements::default(),
            upload_times: Measurements::default(),
            screen_image: Arc::new(egui::ColorImage::filled(
                [WIDTH, HEIGHT],
                egui::Color32::BLACK,
            )),
            texture: None,
            image_generation: 0,
            uploaded_generation: None,
            status: if debug_ui {
                "Loading pcm.gba".to_owned()
            } else {
                "Load a BIOS and ROM".to_owned()
            },
            #[cfg(not(target_arch = "wasm32"))]
            capture_path: std::env::var_os("GBA_CAPTURE_PATH").map(Into::into),
            #[cfg(not(target_arch = "wasm32"))]
            capture_requested: false,
            rom_sender,
            rom_receiver,
            load_generation: 0,
            picker_open: false,
        };
        #[cfg(target_arch = "wasm32")]
        crate::input::browser::set_bindings(&app.settings.preferences.bindings);
        if debug_ui {
            app.load_rom_bytes("pcm.gba", PCM_ROM);
        }
        app
    }

    /// One set of predicates serves presentation and action entry points.
    /// Temporary blockers never change the user's explicit pause choice.
    fn operation_pending(&self) -> bool {
        self.picker_open
            || self.bios_picker_open
            || self.restoring_save
            || self.save_import_open
            || self.pending_rom.is_some()
            || self.pending_bios.is_some()
            || self.close_pending()
    }

    fn operation(&self) -> Operation {
        if self.close_pending() {
            Operation::Closing
        } else if self.bios_picker_open {
            Operation::ReadingBios
        } else if self.picker_open {
            Operation::ReadingRom
        } else if self.restoring_save {
            Operation::RestoringSave
        } else if self.save_import_open {
            Operation::ImportingSave
        } else if self.pending_bios.is_some() {
            Operation::ReplacingBios
        } else if self.pending_rom.is_some() {
            if self
                .pending_rom
                .as_ref()
                .is_some_and(|pending| pending.requires_bios)
                && self.bios_hash.is_none()
            {
                Operation::AwaitingBios
            } else {
                Operation::ReplacingRom
            }
        } else {
            Operation::Idle
        }
    }

    fn can_pause(&self) -> bool {
        self.loaded && !self.operation_pending() && !self.execution_faulted
    }

    fn can_reset(&self) -> bool {
        self.loaded && !self.operation_pending() && !self.storage.busy
    }

    fn can_pick_rom(&self) -> bool {
        !self.picker_open
            && !self.bios_picker_open
            && self.pending_rom.is_none()
            && self.pending_bios.is_none()
            && !self.save_import_open
            && !self.close_pending()
    }

    fn can_pick_bios(&self) -> bool {
        !self.bios_picker_open
            && !self.picker_open
            && !self.restoring_save
            && !self.save_import_open
            && !self.close_pending()
            && self.pending_bios.is_none()
            && (!self.loaded
                || self
                    .pending_rom
                    .as_ref()
                    .is_none_or(|pending| pending.requires_bios))
    }

    /// Presentation actions use the same predicates as their execution paths.
    fn can_import_save(&self) -> bool {
        self.session.save_status().is_some()
            && self.save_identity.is_some()
            && !self.storage.busy
            && !self.picker_open
            && !self.bios_picker_open
            && !self.save_import_open
            && self.pending_rom.is_none()
            && self.pending_bios.is_none()
            && !self.close_pending()
    }

    fn can_export_save(&self) -> bool {
        self.save_identity.is_some()
            && !self.restoring_save
            && !self.storage.busy
            && self
                .session
                .save_status()
                .is_some_and(|status| status.len > 0)
    }

    fn report_error(&mut self, kind: NoticeKind, message: String) {
        self.status.clone_from(&message);
        self.notice = Some(AppNotice { kind, message });
    }

    /// Records key-up independently of focus-local egui state. Losing focus must
    /// not turn an OS repeat into a fresh gameplay press after keys_down is cleared.
    fn observe_host_keys(&mut self, events: &[egui::Event]) {
        for event in events {
            if let egui::Event::Key {
                key,
                pressed,
                modifiers,
                ..
            } = event
                && let Some(slot) = self
                    .settings
                    .preferences
                    .bindings
                    .iter()
                    .position(|binding| binding == key)
            {
                self.host_keys_down[slot] = *pressed;
                if !pressed {
                    self.suppressed_keys[slot] = false;
                } else if !modifiers.is_none() {
                    self.suppressed_keys[slot] = true;
                }
            }
        }
    }

    /// Suppression survives suspension and reset until an actual key-up. This
    /// bounded state never advances the machine or allocates an input collection.
    fn block_host_keys(&mut self) {
        for (blocked, held) in self.suppressed_keys.iter_mut().zip(self.host_keys_down) {
            *blocked |= held;
        }
        self.session.release_all_buttons();
    }

    /// Invalidate input submitted by earlier logic when a widget takes ownership.
    fn own_ui_keys(&mut self, ctx: &egui::Context) {
        ctx.input(|input| {
            self.observe_host_keys(&input.events);
            for (held, key) in self
                .host_keys_down
                .iter_mut()
                .zip(self.settings.preferences.bindings)
            {
                *held |= input.key_down(key);
            }
        });
        self.block_host_keys();
    }

    fn keyboard_ui_owned(&self, ctx: &egui::Context) -> bool {
        self.settings_open
            || self.capture.is_some()
            || egui::Popup::is_any_open(ctx)
            || ctx.egui_wants_keyboard_input()
    }

    /// Runs before egui resolves navigation/activation. Mapped gameplay presses
    /// reach our fixed held-state map but never widgets. Releases remain available
    /// to egui so a key pressed while UI-owned cannot leave its key state stuck.
    fn prepare_host_input(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        self.observe_host_keys(&raw.events);
        #[cfg(not(target_arch = "wasm32"))]
        let visible = !raw.viewport().minimized.unwrap_or(false);
        #[cfg(target_arch = "wasm32")]
        let visible = web_sys::window()
            .and_then(|window| window.document())
            .is_some_and(|document| !document.hidden());
        let interrupted = raw
            .events
            .iter()
            .any(|event| matches!(event, egui::Event::WindowFocused(false)));
        #[cfg(target_arch = "wasm32")]
        let interrupted = {
            let revision = crate::input::browser::lifecycle_revision();
            let changed = revision != self.browser_revision;
            self.browser_revision = revision;
            changed || interrupted
        };
        // Blur and return can share a single batch (or no browser animation frame).
        // Enter the existing inactive boundary once before restoring current facts.
        if interrupted {
            self.sync_execution(false, visible);
            self.update_browser_input(false);
        }
        if self.host_focused != raw.focused || self.host_visible != visible {
            self.sync_execution(raw.focused, visible);
        }
        let running = self.execution_allowed();
        let modifiers = raw
            .events
            .iter()
            .rev()
            .find_map(|event| match event {
                egui::Event::ModifiersChanged(modifiers)
                | egui::Event::Key { modifiers, .. }
                | egui::Event::PointerButton { modifiers, .. } => Some(*modifiers),
                _ => None,
            })
            .unwrap_or_else(|| ctx.input(|input| input.modifiers));
        let mut pointer_ui_event = false;
        for event in &raw.events {
            if let egui::Event::PointerButton { pos, pressed, .. } = event {
                let outside_game = !self.game_rect.is_some_and(|rect| rect.contains(*pos));
                pointer_ui_event |= self.pointer_ui_owned || outside_game;
                self.pointer_ui_owned = *pressed && outside_game;
            }
        }
        self.input_owner = if self.capture.is_some() {
            InputOwner::Rebinding
        } else if !running
            || self.keyboard_ui_owned(ctx)
            || pointer_ui_event
            || self.pointer_ui_owned
            || !modifiers.is_none()
        {
            InputOwner::Ui
        } else {
            InputOwner::Gameplay
        };
        if self.input_owner == InputOwner::Rebinding {
            self.stage_binding_capture(ctx, raw);
        } else if self.input_owner == InputOwner::Gameplay {
            let keys = &self.settings.preferences.bindings;
            raw.events.retain(|event| {
                !matches!(event,
                egui::Event::Key { key, pressed: true, modifiers, .. }
                if modifiers.is_none() && keys.contains(key))
            });
        } else {
            self.block_host_keys();
        }
        self.update_browser_input(self.input_owner == InputOwner::Gameplay);
    }

    /// Capture must run before egui chooses Tab/arrow focus targets or keyboard
    /// activation. A single result is applied in logic after begin-pass; key-ups
    /// remain visible to egui to retire activation-key state safely.
    fn stage_binding_capture(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        let Some(capture) = &mut self.capture else {
            return;
        };
        for event in &mut raw.events {
            if let egui::Event::Key {
                key,
                pressed,
                repeat,
                ..
            } = event
                && let Some(slot) = egui::Key::ALL.iter().position(|candidate| candidate == key)
            {
                if *pressed {
                    *repeat |= self.capture_keys_down[slot];
                }
                self.capture_keys_down[slot] = *pressed;
            }
        }
        let pointer_down = raw
            .events
            .iter()
            .rev()
            .find_map(|event| {
                if let egui::Event::PointerButton { pressed, .. } = event {
                    Some(*pressed)
                } else {
                    None
                }
            })
            .unwrap_or_else(|| ctx.input(|input| input.pointer.any_down()));
        let result = capture.update(
            &raw.events,
            !self.capture_keys_down.contains(&true) && !pointer_down,
        );
        let held = if let CaptureResult::Key(key) = result {
            egui::Key::ALL
                .iter()
                .position(|candidate| *candidate == key)
                .is_some_and(|slot| self.capture_keys_down[slot])
        } else {
            false
        };
        self.pending_capture = Some((result, held));
        self.block_host_keys();
        raw.events
            .retain(|event| !matches!(event, egui::Event::Key { pressed: true, .. }));
    }

    /// The page guard changes only on ownership transitions. It cancels browser
    /// defaults for mapped keys on the focused canvas, never for other page input.
    fn update_browser_input(&mut self, gameplay: bool) {
        #[cfg(target_arch = "wasm32")]
        if self.browser_gameplay != gameplay {
            self.browser_gameplay = gameplay;
            crate::input::browser::set_gameplay(gameplay);
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = gameplay;
    }

    /// Capture starts from a UI-owned frame and immediately invalidates logical
    /// input submitted by earlier logic. Activation events are not processed here.
    fn begin_binding_capture(&mut self, ctx: &egui::Context, slot: usize) {
        self.own_ui_keys(ctx);
        let released = !self.host_keys_down.contains(&true)
            && ctx.input(|input| input.keys_down.is_empty() && !input.pointer.any_down());
        self.capture = Some(Capture::new(slot, released));
        self.pending_capture = None;
        ctx.input(|input| {
            self.capture_keys_down = std::array::from_fn(|index| {
                let key = egui::Key::ALL[index];
                input.key_down(key)
                    || self
                        .settings
                        .preferences
                        .bindings
                        .iter()
                        .position(|binding| *binding == key)
                        .is_some_and(|slot| self.host_keys_down[slot])
            });
        });
        self.capture_notice = None;
    }

    /// Rebuild presentation and suppression together after an atomic edit. Slots
    /// can exchange host keys, so suppression must be resampled for the new map.
    fn bindings_changed(&mut self, ctx: &egui::Context, previous: [egui::Key; 10]) {
        let held = self.host_keys_down;
        ctx.input(|input| {
            self.host_keys_down = std::array::from_fn(|slot| {
                let key = self.settings.preferences.bindings[slot];
                input.key_down(key)
                    || previous
                        .iter()
                        .position(|old| *old == key)
                        .is_some_and(|old| held[old])
            });
        });
        self.binding_labels = binding_labels(&self.settings.preferences.bindings);
        #[cfg(target_arch = "wasm32")]
        crate::input::browser::set_bindings(&self.settings.preferences.bindings);
        self.suppressed_keys = [false; 10];
        self.own_ui_keys(ctx);
    }

    /// Capture owns input while settings suspend execution. A completed/cancelled
    /// event stays ineligible for gameplay until released, including on UI close.
    fn poll_binding_capture(&mut self, ctx: &egui::Context) {
        if self.capture.is_none() {
            return;
        }
        ctx.input(|input| self.observe_host_keys(&input.events));
        let Some(capture) = &mut self.capture else {
            return;
        };
        let slot = capture.slot;
        let host_released = !self.host_keys_down.contains(&true);
        let staged = self.pending_capture.take();
        let (result, captured_held) = if let Some(staged) = staged {
            staged
        } else {
            let result = ctx.input(|input| {
                capture.update(
                    &input.events,
                    host_released && input.keys_down.is_empty() && !input.pointer.any_down(),
                )
            });
            let held = if let CaptureResult::Key(key) = result {
                ctx.input(|input| input.key_down(key))
            } else {
                false
            };
            (result, held)
        };
        match result {
            CaptureResult::Waiting => {}
            CaptureResult::Unsupported => {
                self.capture_notice = Some("Use a supported single key without modifiers; function keys and punctuation are reserved or unsupported.".into());
            }
            CaptureResult::Cancelled => {
                self.capture = None;
                self.capture_notice = Some("Binding cancelled.".into());
            }
            CaptureResult::Key(key) => {
                let previous = self.settings.preferences.bindings;
                let changed = previous[slot] != key;
                if let Ok(other) =
                    crate::input::remap(&mut self.settings.preferences.bindings, slot, key)
                {
                    self.capture_notice = Some(if let Some(other) = other {
                        format!(
                            "Swapped {} and {} bindings.",
                            button_name(BINDING_BUTTONS[slot]),
                            button_name(BINDING_BUTTONS[other])
                        )
                    } else {
                        format!(
                            "{} bound to {}.",
                            button_name(BINDING_BUTTONS[slot]),
                            key.name()
                        )
                    });
                    if changed {
                        self.bindings_changed(ctx, previous);
                    }
                    // The captured press was withheld from egui. Preserve its
                    // real held state across an unused-key assignment as well.
                    self.host_keys_down[slot] = captured_held;
                    self.suppressed_keys[slot] |= captured_held;
                }
                self.capture = None;
            }
        }
        self.own_ui_keys(ctx);
        // egui already recorded held/released state. Remove capture-owned events
        // before drawing so Enter/Space cannot activate retained widget focus and
        // Escape cancels capture without also dismissing Settings.
        ctx.input_mut(|input| {
            input
                .events
                .retain(|event| !matches!(event, egui::Event::Key { .. }))
        });
    }

    fn import_cartridge_save(&mut self, ctx: &egui::Context) {
        if !self.can_import_save() {
            return;
        }
        if let Some(identity) = self.save_identity {
            self.own_ui_keys(ctx);
            self.storage.import(ctx, identity, self.save_generation);
            self.save_import_open = self.storage.busy;
            self.sync_execution(self.host_focused, self.host_visible);
        }
    }

    fn export_cartridge_save(&mut self, ctx: &egui::Context) {
        if !self.can_export_save() {
            return;
        }
        if let Some(identity) = self.save_identity
            && let Some(image) = self.session.save_image()
        {
            self.own_ui_keys(ctx);
            self.storage
                .export(ctx, identity, self.save_generation, image);
        }
    }

    /// Temporary execution blockers and explicit pause are independent. Use the
    /// same predicate for input routing, browser ownership and idle repaint policy.
    fn session_active(&self) -> bool {
        self.loaded
            && self.host_focused
            && self.host_visible
            && !self.operation_pending()
            && !self.settings_open
    }

    fn execution_allowed(&self) -> bool {
        self.session_active() && !self.user_paused && !self.execution_faulted
    }

    /// Synchronizes session flags through its existing reanchoring APIs. The
    /// operation gate is constant-size and never performs guest or storage work.
    fn sync_execution(&mut self, focused: bool, visible: bool) -> bool {
        self.host_focused = focused;
        self.host_visible = visible;
        if !(focused && visible) {
            self.capture = None;
            self.pending_capture = None;
            self.pointer_ui_owned = false;
        }
        let paused = self.user_paused
            || self.restoring_save
            || self.execution_faulted
            || self.close_pending();
        if self.session.paused() != paused {
            self.session.toggle_pause();
            self.audio.clear();
        }
        let active = self.session_active();
        self.session.set_active(active);
        if !active || paused {
            self.block_host_keys();
        }
        self.audio.set_playing(active && !paused);
        active && !paused
    }

    fn toggle_user_pause(&mut self) {
        if self.can_pause() {
            self.user_paused = !self.user_paused;
            self.replay_deadline = None;
            self.sync_execution(self.host_focused, self.host_visible);
        }
    }

    /// Reads supplied firmware off the UI loop on native and through a local
    /// browser future on WASM. Cancellation leaves the installed image intact.
    fn pick_bios(&mut self, ctx: &egui::Context) {
        if !self.can_pick_bios() {
            return;
        }
        self.notice = None;
        self.load_generation = self.load_generation.wrapping_add(1);
        let generation = self.load_generation;
        self.bios_picker_open = true;
        self.sync_execution(self.host_focused, self.host_visible);
        let sender = self.bios_sender.clone();
        let ctx = ctx.clone();
        let selection = rfd::AsyncFileDialog::new()
            .set_title("Load GBA BIOS (16 KiB)")
            .add_filter("BIOS", &["bin", "rom"])
            .pick_file();
        let task = async move {
            let result = if let Some(file) = selection.await {
                #[cfg(not(target_arch = "wasm32"))]
                let bytes = std::fs::read(file.path()).map_err(|error| error.to_string());
                #[cfg(target_arch = "wasm32")]
                let bytes = wasm_bindgen_futures::JsFuture::from(file.inner().array_buffer())
                    .await
                    .map(|buffer| js_sys::Uint8Array::new(&buffer).to_vec())
                    .map_err(|error| format!("{error:?}"));
                Some((file.file_name(), bytes))
            } else {
                None
            };
            let _ = sender.send(BiosRead { generation, result });
            ctx.request_repaint();
        };
        #[cfg(not(target_arch = "wasm32"))]
        if let Err(error) = std::thread::Builder::new()
            .name("bios-picker".into())
            .spawn(move || pollster::block_on(task))
        {
            self.bios_picker_open = false;
            self.report_error(NoticeKind::Bios, format!("BIOS picker failed: {error}"));
        }
        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(task);
    }

    #[cfg(test)]
    fn load_bios_bytes(&mut self, bytes: &[u8]) {
        self.load_bios_request("test BIOS.bin", bytes);
    }

    /// Validate before staging; the current firmware and session remain installed
    /// while cartridge restoration or an outstanding save write blocks replacement.
    fn load_bios_request(&mut self, name: &str, bytes: &[u8]) {
        if bytes.len() != 16384 {
            self.report_error(
                NoticeKind::Bios,
                format!(
                    "BIOS must be 16 KiB (16384 bytes); received {} bytes.",
                    bytes.len()
                ),
            );
            return;
        }
        if self.loaded
            && (self.storage.busy || self.restoring_save || self.session.persistence_dirty())
        {
            self.pending_bios = Some(PendingBios {
                name: name.into(),
                bytes: bytes.to_vec(),
            });
            self.sync_execution(self.host_focused, self.host_visible);
            return;
        }
        self.install_bios(name, bytes);
    }

    /// Commits through the existing session operation after the app's save barrier.
    /// Firmware names/readiness change only on successful installation.
    fn install_bios(&mut self, name: &str, bytes: &[u8]) {
        match self.session.load_bios(bytes) {
            Ok(()) => {
                self.audio.clear();
                self.host_origin = Instant::now();
                self.replay_deadline = None;
                self.last_guest_generation = 0;
                Arc::make_mut(&mut self.screen_image)
                    .pixels
                    .fill(egui::Color32::BLACK);
                self.image_generation = self.image_generation.wrapping_add(1);
                self.bios_hash = Some(format!("{:x}", Sha256::digest(bytes)));
                self.bios_name = Some(name.into());
                self.execution_faulted = false;
                self.notice = None;
                self.sync_execution(self.host_focused, self.host_visible);
                self.status = if self.loaded {
                    "BIOS replaced; game restarted"
                } else {
                    "BIOS ready; load a ROM to start"
                }
                .into();
            }
            Err(error) => self.report_error(NoticeKind::Bios, format!("BIOS load failed: {error}")),
        }
    }

    /// Cancellation removes only the queued replacement, never an issued save write.
    fn cancel_replacement(&mut self) {
        if let Some(pending) = self.pending_rom.take() {
            self.restore_backup_override(pending.backup_override);
        }
        self.pending_bios = None;
        self.sync_execution(self.host_focused, self.host_visible);
    }

    /// Byte identity confines controlled startup to the shipped diagnostics.
    /// Every other cartridge must pass through the supplied BIOS reset path.
    fn controlled_rom(bytes: &[u8]) -> bool {
        if DISPLAY_DIAGNOSTIC_ROMS.contains(&bytes) {
            return true;
        }
        [
            FLASH_DIAGNOSTIC_ROM,
            BANKED_DIAGNOSTIC_ROM,
            EEPROM512_ROM,
            EEPROM8K_ROM,
            BANKED_ROM,
            FLASH_ROM,
            SRAM_ROM,
            RTC_ROM,
            BUTTONS_ROM,
            PALETTE_ROM,
            CALCULATIONS_ROM,
            COPY_ROM,
            COUNTER_ROM,
            ARM_DIAGNOSTIC_ROM,
            THUMB_DIAGNOSTIC_ROM,
            TILED_ROM,
            STRIPES_ROM,
            KEYPAD_OR_ROM,
            KEYPAD_AND_ROM,
            VBLANK_ROM,
            NOISE_ROM,
            WAVE_ROM,
            PULSE_ROM,
            PCM_ROM,
            DMA_ROM,
            IRQ_SPRITES_ROM,
            SPRITES_ROM,
            MEMORY_ROM,
        ]
        .contains(&bytes)
    }

    /// Starts one asynchronous picker from the user's click. Native dialog/read
    /// work runs on a worker; browser work stays on its local executor.
    fn pick_rom(&mut self, ctx: &egui::Context) {
        if !self.can_pick_rom() {
            return;
        }
        let backup_override = self.backup_override.take();
        self.load_generation = self.load_generation.wrapping_add(1);
        let generation = self.load_generation;
        self.picker_open = true;
        self.sync_execution(self.host_focused, self.host_visible);
        let sender = self.rom_sender.clone();
        let ctx = ctx.clone();
        let dialog = rfd::AsyncFileDialog::new()
            .set_title("Load GBA ROM")
            .add_filter("GBA ROM", &["gba"]);
        // Construct the future during the click so browsers retain user activation.
        let selection = dialog.pick_file();
        #[cfg(not(target_arch = "wasm32"))]
        let fallback_override = backup_override;
        let task = async move {
            let result = if let Some(file) = selection.await {
                let name = file.file_name();
                #[cfg(not(target_arch = "wasm32"))]
                let bytes = std::fs::read(file.path()).map_err(|error| error.to_string());
                #[cfg(target_arch = "wasm32")]
                let bytes = wasm_bindgen_futures::JsFuture::from(file.inner().array_buffer())
                    .await
                    .map(|buffer| js_sys::Uint8Array::new(&buffer).to_vec())
                    .map_err(|error| format!("{error:?}"));
                Some((name, bytes))
            } else {
                None
            };
            let _ = sender.send(RomRead {
                generation,
                backup_override,
                result,
            });
            ctx.request_repaint();
        };
        #[cfg(not(target_arch = "wasm32"))]
        if let Err(error) = std::thread::Builder::new()
            .name("rom-picker".into())
            .spawn(move || pollster::block_on(task))
        {
            self.picker_open = false;
            self.restore_backup_override(fallback_override);
            self.report_error(NoticeKind::Rom, format!("ROM picker failed: {error}"));
        }
        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(task);
    }

    /// Accepts bytes from built-in scenes, picked files and dropped files.
    fn load_rom_bytes(&mut self, name: &str, bytes: &[u8]) {
        let backup_override = self.backup_override.take();
        self.load_rom_request(name, bytes, backup_override);
    }

    /// Keeps the override with this request while save persistence delays installation.
    fn load_rom_request(
        &mut self,
        name: &str,
        bytes: &[u8],
        backup_override: Option<gba_session::BackupType>,
    ) {
        self.load_generation = self.load_generation.wrapping_add(1);
        self.picker_open = false;
        self.bios_picker_open = false;
        self.pending_bios = None;
        self.notice = None;
        if !Self::controlled_rom(bytes) && self.bios_hash.is_none() {
            // Validate through the existing loader without installing into the
            // live owner. This bounded startup-only work never runs per frame.
            let mut validation = Session::new();
            if let Err(error) = validation.load_rom_with_backup(bytes, backup_override) {
                self.restore_backup_override(backup_override);
                self.report_error(NoticeKind::Rom, format!("ROM load failed: {error}"));
                return;
            }
            self.pending_rom = Some(PendingRom {
                name: name.to_owned(),
                bytes: bytes.to_vec(),
                backup_override,
                requires_bios: true,
            });
            self.status = "ROM ready; Load BIOS to start".into();
            self.sync_execution(self.host_focused, self.host_visible);
            return;
        }
        // Replacement is a save barrier. Retain the old machine until its last
        // dirty revision is durable; a failed write remains retryable/exportable.
        if self.storage.busy || self.restoring_save || self.session.persistence_dirty() {
            self.pending_rom = Some(PendingRom {
                name: name.to_owned(),
                bytes: bytes.to_vec(),
                backup_override,
                requires_bios: false,
            });
            self.status = "Saving current game before loading another ROM".into();
            self.sync_execution(self.host_focused, self.host_visible);
            return;
        }
        self.install_rom(name, bytes, backup_override);
    }

    /// Installs a cartridge only after the preceding session's save barrier.
    fn install_rom(
        &mut self,
        name: &str,
        bytes: &[u8],
        requested_override: Option<gba_session::BackupType>,
    ) {
        self.load_generation = self.load_generation.wrapping_add(1);
        self.picker_open = false;
        // The pinned stripes file ends at its idle branch. Two unexecuted words
        // allow this loader's ARM pipeline look-ahead without changing the asset.
        let mut mapped;
        let bytes_to_load = if bytes == STRIPES_ROM {
            mapped = bytes.to_vec();
            mapped.extend_from_slice(&[0; 8]);
            mapped.as_slice()
        } else {
            bytes
        };
        // The pinned homebrew Flash test has padded SDK strings without a
        // version. Its explicit fixture configuration uses the common override
        // path; picked games still rely on detector evidence or the user's choice.
        let backup_override = requested_override.or_else(|| {
            if bytes == BANKED_DIAGNOSTIC_ROM {
                Some(gba_session::BackupType::Flash128)
            } else {
                (bytes == FLASH_DIAGNOSTIC_ROM).then_some(gba_session::BackupType::Flash64)
            }
        });
        let result = if Self::controlled_rom(bytes) {
            self.session
                .load_rom_with_backup(bytes_to_load, backup_override)
        } else {
            self.session
                .boot_rom_with_bios(bytes_to_load, backup_override)
        };
        match result {
            Ok(()) => {
                self.save_generation = self.save_generation.wrapping_add(1);
                self.save_identity = (self.session.save_status().is_some()
                    || self.session.rtc_image().is_some())
                .then(|| crate::saves::Identity::of(bytes));
                self.restoring_save = self.save_identity.is_some();
                self.storage.failed = false;
                self.save_failure = None;
                self.execution_faulted = false;
                self.notice = None;
                self.pending_rom = None;
                if self.restoring_save || self.user_paused {
                    self.session.toggle_pause();
                }
                if bytes == ARM_DIAGNOSTIC_ROM
                    || bytes == THUMB_DIAGNOSTIC_ROM
                    || bytes == MEMORY_ROM
                    || bytes == FLASH_DIAGNOSTIC_ROM
                    || bytes == BANKED_DIAGNOSTIC_ROM
                    || bytes == KEYPAD_OR_ROM
                    || bytes == KEYPAD_AND_ROM
                    || bytes == VBLANK_ROM
                    || bytes == IRQ_SPRITES_ROM
                    || bytes == DMA_ROM
                    || bytes == PCM_ROM
                    || bytes == PULSE_ROM
                    || bytes == WAVE_ROM
                    || bytes == NOISE_ROM
                {
                    self.session.enable_test_firmware();
                }
                self.audio.clear();
                self.rom_name = name.to_owned();
                self.loaded = true;
                self.status = format!("Loaded {name}");
                self.host_origin = Instant::now();
                self.replay_deadline = None;
                self.last_guest_generation = 0;
                Arc::make_mut(&mut self.screen_image)
                    .pixels
                    .fill(egui::Color32::BLACK);
                self.image_generation = self.image_generation.wrapping_add(1);
                self.core_times = Measurements::default();
                self.conversion_times = Measurements::default();
                self.upload_times = Measurements::default();
            }
            Err(error) => {
                self.restore_backup_override(requested_override);
                self.report_error(NoticeKind::Rom, format!("ROM load failed: {error}"));
            }
        }
    }

    /// Restores a request's override only when the user has not chosen a newer one.
    fn restore_backup_override(&mut self, requested_override: Option<gba_session::BackupType>) {
        if self.backup_override.is_none() {
            self.backup_override = requested_override;
        }
    }

    /// Browser shutdown has no asynchronous completion guarantee; native close
    /// keeps the viewport alive until the save barrier completes.
    fn close_pending(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.close_when_saved
        }
        #[cfg(target_arch = "wasm32")]
        {
            false
        }
    }

    /// Processes definitive storage completions before issuing the next operation.
    /// Generation and identity checks prevent stale work from acknowledging a new
    /// machine; revision checks in the core preserve edits made during a write.
    fn poll_save_storage(&mut self, ctx: &egui::Context) {
        if let Some(completion) = self.storage.poll() {
            self.apply_save_completion(completion);
        }
        if !self.storage.busy && !self.storage.failed {
            if self.restoring_save {
                if let Some(identity) = self.save_identity {
                    self.storage.load(ctx, identity, self.save_generation);
                }
            } else if self.session.persistence_dirty() {
                if let Some(identity) = self.save_identity {
                    self.storage.write(
                        ctx,
                        identity,
                        self.save_generation,
                        self.session.save_image(),
                        self.session.rtc_image(),
                    );
                }
            } else if let Some(pending) = self.pending_bios.take() {
                self.install_bios(&pending.name, &pending.bytes);
            } else if self
                .pending_rom
                .as_ref()
                .is_some_and(|pending| self.bios_hash.is_some() || !pending.requires_bios)
                && let Some(pending) = self.pending_rom.take()
            {
                self.install_rom(&pending.name, &pending.bytes, pending.backup_override);
            }
        }
        // Worker-dispatch failures are host outcomes too. Retain their actual
        // reason once; an export cannot replace it with download feedback.
        if self.storage.failed && self.save_failure.is_none() {
            self.remember_save_failure(if self.restoring_save {
                NoticeKind::SaveRestore
            } else {
                NoticeKind::SaveWrite
            });
        }
        if self.storage.busy {
            ctx.request_repaint_after(Duration::from_millis(20));
        }
    }

    fn remember_save_failure(&mut self, kind: NoticeKind) {
        self.save_failure = Some(AppNotice {
            kind,
            message: self.storage.status.clone(),
        });
    }

    /// Applies one definitive result only to its matching installed cartridge.
    /// Presentation feedback is separate from storage acknowledgement/barriers.
    fn apply_save_completion(&mut self, completion: crate::saves::Completion) {
        if self.save_identity.as_ref() != Some(&completion.identity)
            || self.save_generation != completion.generation
        {
            return;
        }
        use crate::saves::Outcome;
        match completion.outcome {
            Outcome::Loaded(Ok(bytes)) => {
                let result = (|| {
                    if let Some(record) = &bytes {
                        if let Some(rtc) = &record.rtc {
                            self.session.load_rtc(rtc)?;
                        }
                        if let Some(backup) = &record.backup {
                            self.session.load_save(backup)?;
                        }
                    }
                    Ok::<(), &'static str>(())
                })();
                match result {
                    Ok(()) => {
                        self.restoring_save = false;
                        self.save_failure = None;
                        self.storage.status = if bytes.is_some() {
                            "Backup restored"
                        } else {
                            "No stored backup; new cartridge"
                        }
                        .into();
                        self.sync_execution(self.host_focused, self.host_visible);
                    }
                    Err(error) => {
                        self.storage.failed = true;
                        self.storage.status =
                            format!("Save restoration failed: {error}; guest remains paused");
                        self.remember_save_failure(NoticeKind::SaveRestore);
                    }
                }
            }
            Outcome::Loaded(Err(error)) => {
                self.storage.failed = true;
                self.storage.status =
                    format!("Initial backup load failed: {error}; guest remains paused");
                self.remember_save_failure(NoticeKind::SaveRestore);
            }
            Outcome::Written(Ok(())) => {
                self.session.acknowledge_save(completion.revision);
                if let Some(revision) = completion.rtc_revision {
                    self.session.acknowledge_rtc(revision);
                }
                self.storage.failed = false;
                self.save_failure = None;
                self.storage.status = if self.session.persistence_dirty() {
                    "Stored snapshot; newer backup changes remain pending".into()
                } else {
                    format!("Saved revision {}", completion.revision)
                };
            }
            Outcome::Written(Err(error)) => {
                self.storage.failed = true;
                self.storage.status =
                    format!("Save failed: {error}; bytes pending — Retry or Export");
                self.remember_save_failure(NoticeKind::SaveWrite);
            }
            Outcome::Imported(Ok(Some(bytes))) => {
                self.save_import_open = false;
                match self.session.import_save(&bytes) {
                    Ok(()) => {
                        self.storage.failed = false;
                        self.save_failure = None;
                        self.restoring_save = false;
                        // Restart guest registers so the imported score is read
                        // from cartridge bytes rather than overwritten by the old
                        // score still held in CPU registers. Keep execution paused.
                        self.reset_demo();
                        self.user_paused = true;
                        self.sync_execution(self.host_focused, self.host_visible);
                        self.storage.status =
                            "Imported; persistence pending — Resume to display".into();
                        self.notice = Some(AppNotice {
                            kind: NoticeKind::SaveImport,
                            message: self.storage.status.clone(),
                        });
                    }
                    Err(error) => {
                        self.storage.status = format!("Import rejected: {error}");
                        self.report_error(NoticeKind::SaveImport, self.storage.status.clone());
                    }
                }
            }
            Outcome::Imported(Ok(None)) => {
                self.save_import_open = false;
                self.storage.status = "Import cancelled".into();
            }
            Outcome::Imported(Err(error)) => {
                self.save_import_open = false;
                self.storage.status = format!("Import rejected: {error}");
                self.report_error(NoticeKind::SaveImport, self.storage.status.clone());
            }
            Outcome::Exported(Ok(true)) => {
                self.storage.status =
                        "Export prepared. Check the saved file/download; automatic storage acknowledgement is unchanged.".into();
                self.notice = Some(AppNotice {
                    kind: NoticeKind::SaveExport,
                    message: self.storage.status.clone(),
                });
            }
            Outcome::Exported(Ok(false)) => self.storage.status = "Export cancelled".into(),
            Outcome::Exported(Err(error)) => {
                self.storage.status = format!("Export failed: {error}");
                self.report_error(NoticeKind::SaveExport, self.storage.status.clone());
            }
        }
    }

    /// Converts the core's row-major BGR555 pixels into the shared egui image.
    fn copy_framebuffer_to_image(&mut self) {
        let framebuffer = self.session.framebuffer();

        let image = Arc::make_mut(&mut self.screen_image);

        for (destination, &pixel) in image.pixels.iter_mut().zip(framebuffer) {
            let red = expand_five_bit(pixel & 0x1f);
            let green = expand_five_bit((pixel >> 5) & 0x1f);
            let blue = expand_five_bit((pixel >> 10) & 0x1f);

            *destination = egui::Color32::from_rgb(red, green, blue);
        }
    }

    /// Submit only logical transitions from the bounded host map. Raw gameplay
    /// events have already been removed before egui's navigation pass.
    fn poll_keyboard(&mut self, ctx: &egui::Context) {
        ctx.input(|input| {
            self.observe_host_keys(&input.events);
            for (slot, button) in BINDING_BUTTONS.into_iter().enumerate() {
                self.session.set_button(
                    button,
                    self.host_keys_down[slot]
                        && !self.suppressed_keys[slot]
                        && input.modifiers.is_none(),
                );
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
                Arc::clone(&self.screen_image),
                egui::TextureOptions::NEAREST,
            ));
            self.uploaded_generation = Some(self.image_generation);
        } else if changed {
            if let Some(texture) = &mut self.texture {
                texture.set(
                    Arc::clone(&self.screen_image),
                    egui::TextureOptions::NEAREST,
                );
            }
            self.uploaded_generation = Some(self.image_generation);
        }
        if changed {
            self.upload_times
                .push(start.elapsed().as_secs_f64() * 1000.0);
        }
    }

    /// Restarts execution with cartridge data and explicit pause intent retained.
    fn reset_demo(&mut self) {
        if !self.can_reset() {
            return;
        }
        self.block_host_keys();
        self.audio.clear();
        self.session.reset();
        self.execution_faulted = false;
        self.notice = None;
        self.sync_execution(self.host_focused, self.host_visible);
        self.host_origin = Instant::now();
        self.replay_deadline = None;
        self.last_guest_generation = 0;
        Arc::make_mut(&mut self.screen_image)
            .pixels
            .fill(egui::Color32::BLACK);
        self.status = format!("Reset {}", self.rom_name);
        self.image_generation = self.image_generation.wrapping_add(1);
    }

    /// Retains all fixture controls and detailed diagnostics for development.
    fn draw_debug_ui(&mut self, ui: &mut egui::Ui) {
        self.sync_texture(ui.ctx());
        #[cfg(not(target_arch = "wasm32"))]
        if self.capture_path.is_some() && !self.capture_requested && self.core_times.count == 120 {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
            self.capture_requested = true;
        }

        ui.heading("Lycan — Diagnostics");
        self.draw_save_controls(ui);
        ui.horizontal_wrapped(|ui| {
            if ui.add_enabled(self.can_pick_bios(), egui::Button::new("Load BIOS")).clicked() {
                self.pick_bios(ui.ctx());
            }
            ui.label(self.bios_hash.as_ref().map_or_else(
                || "BIOS: not loaded".to_owned(),
                |hash| format!("BIOS: 16384 bytes, SHA-256 {hash}"),
            ));
            if ui.add_enabled(self.can_pick_rom(), egui::Button::new("Load ROM")).clicked() {
                self.pick_rom(ui.ctx());
            }
            let pause_label = if self.user_paused {
                "Resume"
            } else {
                "Pause"
            };

            if ui.add_enabled(self.can_pause(), egui::Button::new(pause_label)).clicked() {
                self.toggle_user_pause();
                ui.ctx().request_repaint();
            }
            if ui.add_enabled(self.can_reset(), egui::Button::new("Reset")).clicked() {
                self.reset_demo();
                ui.ctx().request_repaint();
            }
            if ui.button("Load EEPROM 512-byte score").clicked() {
                self.load_rom_bytes("eeprom512-score.gba", EEPROM512_ROM);
            }
            if ui.button("Load EEPROM 8-KiB score").clicked() {
                self.load_rom_bytes("eeprom8k-score.gba", EEPROM8K_ROM);
            }
            if ui.button("Load Flash128 test").clicked() {
                self.load_rom_bytes("flash128.gba", BANKED_DIAGNOSTIC_ROM);
            }
            if ui.button("Load banked Flash128 score").clicked() {
                self.load_rom_bytes("flash-banked-score.gba", BANKED_ROM);
            }
            if ui.button("Load Flash64 test").clicked() {
                self.load_rom_bytes("flash64.gba", FLASH_DIAGNOSTIC_ROM);
                ui.ctx().request_repaint();
            }
            if ui.button("Load Flash64 score").clicked() {
                self.load_rom_bytes("flash-score.gba", FLASH_ROM);
                ui.ctx().request_repaint();
            }
            if ui.button("Load SRAM score").clicked() {
                self.load_rom_bytes("sram.gba", SRAM_ROM);
                ui.ctx().request_repaint();
            }
            if ui.button("Load noise scene").clicked() {
                self.load_rom_bytes("noise.gba", NOISE_ROM);
                self.status = "Noise: paired red/green events swap FIFO clocks; Z triggers; X selects short noise; Backspace isolates noise".to_owned();
            }
            if ui.button("Load wave effect").clicked() {
                self.load_rom_bytes("wave.gba", WAVE_ROM);
                self.status = "Wave: Z selects effect + PCM; X selects bank; Backspace selects 64 digits; Up selects 75%; Down mutes".to_owned();
            }
            if ui.button("Load pulse melody").clicked() {
                self.load_rom_bytes("pulse.gba", PULSE_ROM);
                self.status = "Pulse melody: Z adds PCM; X sweeps; Select changes duty; Up fades; Down mutes; release resets".to_owned();
            }
            if ui.button("Load PCM scene").clicked() {
                self.load_rom_bytes("pcm.gba", PCM_ROM);
                self.status = "PCM: hold Z for a green square and tone; release for red and silence".to_owned();
            }
            if ui.button("Replay PCM input").clicked() {
                self.load_rom_bytes("pcm.gba", PCM_ROM);
                for &(cycle, button, pressed) in PCM_INPUT {
                    if let Err(error) = self.session.set_button_at(cycle, button, pressed) {
                        self.status = format!("PCM replay failed: {error}");
                        return;
                    }
                }
                self.replay_deadline = Some(Cycle(10 * CYCLES_PER_FRAME));
                self.status = "PCM replay: red/silent, green/tone, red/silent".to_owned();
            }
            if ui.button("Load DMA tile scene").clicked() {
                self.load_rom_bytes("dma.gba", DMA_ROM);
                self.status = "DMA tiles: hold Z for green; release for red (VBlank upload)".to_owned();
                ui.ctx().request_repaint();
            }
            if ui.button("Replay DMA input").clicked() {
                self.load_rom_bytes("dma.gba", DMA_ROM);
                for &(cycle, button, pressed) in DMA_INPUT {
                    if let Err(error) = self.session.set_button_at(cycle, button, pressed) {
                        self.status = format!("DMA replay failed: {error}");
                        return;
                    }
                }
                self.replay_deadline = Some(Cycle(10 * CYCLES_PER_FRAME));
                self.status = "DMA replay: red, green, then red; final frame is red".to_owned();
                ui.ctx().request_repaint();
            }
            if ui.button("Load IRQ sprite scene").clicked() {
                self.load_rom_bytes("irq-sprites.gba", IRQ_SPRITES_ROM);
                self.status = "IRQ sprite scene: arrows move; Z changes priority; X selects 2D tiles".to_owned();
                ui.ctx().request_repaint();
            }
            if ui.button("Replay IRQ sprite input").clicked() {
                self.load_rom_bytes("irq-sprites.gba", IRQ_SPRITES_ROM);
                for &(cycle, button, pressed) in SPRITE_INPUT {
                    if let Err(error) = self.session.set_button_at(cycle, button, pressed) {
                        self.status = format!("Replay input failed: {error}");
                        self.user_paused = true;
                        self.sync_execution(self.host_focused, self.host_visible);
                        return;
                    }
                }
                self.replay_deadline = Some(Cycle(20 * CYCLES_PER_FRAME));
                self.status = "Replaying IRQ sprite scene; final player (116, 74)".to_owned();
                ui.ctx().request_repaint();
            }
            for (name, rom) in [("keypad-or.gba", KEYPAD_OR_ROM), ("keypad-and.gba", KEYPAD_AND_ROM)] {
                if ui.button(format!("Replay {name}")).clicked() {
                    self.load_rom_bytes(name, rom);
                    for &(cycle, button, pressed) in KEYPAD_INPUT {
                        if let Err(error) = self.session.set_button_at(cycle, button, pressed) {
                            self.status = format!("Keypad replay failed: {error}");
                            return;
                        }
                    }
                    self.replay_deadline = Some(Cycle(2 * CYCLES_PER_FRAME));
                    self.status = format!("{name}: Right ignored; A wakes OR; A+B wakes AND");
                    ui.ctx().request_repaint();
                }
            }
            if ui.button("Load VBlank IRQ demo").clicked() {
                self.load_rom_bytes("vblank.gba", VBLANK_ROM);
                self.status = "VBlank IRQ: callback changes the red backdrop; guest sleeps between frames".to_owned();
                ui.ctx().request_repaint();
            }
            if ui.button("Load sprite demo").clicked() {
                self.load_rom_bytes("sprites.gba", SPRITES_ROM);
                self.status = "Sprite scene: arrows move; Z puts the player behind BG; X selects 2D tiles".to_owned();
                ui.ctx().request_repaint();
            }
            if ui.button("Replay sprite input").clicked() {
                self.load_rom_bytes("sprites.gba", SPRITES_ROM);
                for &(cycle, button, pressed) in SPRITE_INPUT {
                    if let Err(error) = self.session.set_button_at(cycle, button, pressed) {
                        self.status = format!("Replay input failed: {error}");
                        self.user_paused = true;
                        self.sync_execution(self.host_focused, self.host_visible);
                        return;
                    }
                }
                self.replay_deadline = Some(Cycle(20 * CYCLES_PER_FRAME));
                self.status = "Replaying sprite overlap and layouts; final player (116, 74)".to_owned();
                ui.ctx().request_repaint();
            }
            if ui.button("Load memory diagnostic").clicked() {
                self.load_rom_bytes("memory.gba", MEMORY_ROM);
                self.status = "Memory diagnostic: expected All tests passed".to_owned();
                ui.ctx().request_repaint();
            }
            if ui.button("Load tiled demo").clicked() {
                self.load_rom_bytes("tiled.gba", TILED_ROM);
                self.status = "Tiled scene: arrow keys scroll and wrap the 512×512 background".to_owned();
                ui.ctx().request_repaint();
            }
            if ui.button("Replay tiled input").clicked() {
                self.load_rom_bytes("tiled.gba", TILED_ROM);
                for &(cycle, button, pressed) in TILED_INPUT {
                    if let Err(error) = self.session.set_button_at(cycle, button, pressed) {
                        self.status = format!("Replay input failed: {error}");
                        self.user_paused = true;
                        self.sync_execution(self.host_focused, self.host_visible);
                        return;
                    }
                }
                self.replay_deadline = Some(Cycle(19 * CYCLES_PER_FRAME));
                self.status = "Replaying tiled scroll/wrap; final scroll (2, 0)".to_owned();
                ui.ctx().request_repaint();
            }
            if ui.button("Load stripes reference").clicked() {
                self.load_rom_bytes("stripes.gba", STRIPES_ROM);
                ui.ctx().request_repaint();
            }
            if ui.button("Load copy demo").clicked() {
                self.load_rom_bytes("copy.gba", COPY_ROM);
                self.status = "Copy demo: red upper band and palette-byte-write lower band".to_owned();
                ui.ctx().request_repaint();
            }
            if ui.button("Load counter demo").clicked() {
                self.load_rom_bytes("counter.gba", COUNTER_ROM);
                self.status = "Counter demo: hold Z (A) to fill 16 green cells; Reset clears the counter".to_owned();
                ui.ctx().request_repaint();
            }
            if ui.button("Load ARM tests").clicked() {
                self.load_rom_bytes("arm.gba", ARM_DIAGNOSTIC_ROM);
                self.status = "ARM tests: expect All tests passed; a failure screen gives the first case".to_owned();
                ui.ctx().request_repaint();
            }
            if ui.button("Load Thumb tests").clicked() {
                self.load_rom_bytes("thumb.gba", THUMB_DIAGNOSTIC_ROM);
                self.status = "Thumb tests: expect All tests passed; a failure screen gives the first case".to_owned();
                ui.ctx().request_repaint();
            }
            if ui.button("Load CPU diagnostic").clicked() {
                self.load_rom_bytes("calculations.gba", CALCULATIONS_ROM);
                self.status = "CPU diagnostic: bands 1–16 run from top to bottom; green passes, red fails".to_owned();
                ui.ctx().request_repaint();
            }
            if ui.button("Load palette demo").clicked() {
                self.load_rom_bytes("palette.gba", PALETTE_ROM);
                self.status = "Palette demo: hold Z (A) for page 1; hold X (B) to change its white stripes to red".to_owned();
                ui.ctx().request_repaint();
            }
            if ui.button("Replay square demo").clicked() {
                self.load_rom_bytes("buttons.gba", BUTTONS_ROM);
                for &(cycle, button, pressed) in DEMO_INPUT {
                    if let Err(error) = self.session.set_button_at(cycle, button, pressed) {
                        self.status = format!("Replay input failed: {error}");
                        self.user_paused = true;
                        self.sync_execution(self.host_focused, self.host_visible);
                        return;
                    }
                }
                self.replay_deadline = Some(Cycle(9 * CYCLES_PER_FRAME));
                self.status = "Replaying square demo input".to_owned();
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

            if self.rom_name == "affine-object.gba" {
                ui.label("Sprite: Right rotates; Up enlarges; Z expands bounds; X changes color depth; S changes tile layout; A changes priority; Left clips position.");
            }

            if self.rom_name == "raster.gba"
                && let (Ok(state), Ok(flags)) = (
                    self.session.inspect16(0x0300_0002),
                    self.session.inspect16(0x0300_0004),
                )
            {
                let description = match state {
                    0 => "DMA0: increasing brightness",
                    1 => "DMA3: decreasing brightness",
                    2 => "DMA0 + DMA3: DMA3 final values",
                    _ => "unexpected raster state",
                };

                ui.monospace(format!(
                    "Raster state {state}: {description} | DMA IF snapshot {flags:#06x}"
                ));
                ui.label(
                    "Left/Right change raster state. State 2 should visually match state 1.",
                );
            }

            if self.rom_name.starts_with("affine-mode-")
                && let (
                    Ok(0x00a1),
                    Ok(angle),
                    Ok(scale),
                    Ok(page),
                    Ok(wrap),
                ) = (
                    self.session.inspect16(0x0300_0000),
                    self.session.inspect16(0x0300_0002),
                    self.session.inspect16(0x0300_0004),
                    self.session.inspect16(0x0300_0006),
                    self.session.inspect16(0x0300_0008),
                )
            {
                let scale = match scale {
                    0x0100 => "1x source step",
                    0x0200 => "2x source step",
                    _ => "unexpected",
                };

                ui.monospace(format!(
                    "Affine: angle={}° | {scale} | page={} | wrap={}",
                    angle * 90,
                    if page != 0 { 1 } else { 0 },
                    wrap != 0,
                ));
            }
        });

        self.draw_audio_controls(ui, 180.0, !self.settings_open);
        ui.label(self.audio.status());
        let (produced, dropped, empty) = self.session.pcm_counters();
        ui.label(format!("Core PCM: 32768 Hz | produced {produced} | staging drops {dropped} | empty FIFO {empty}"));
        if self.rom_name == "noise.gba" {
            ui.label("Red/green events: noise + swapped FIFO clocks | Z: alternate event | X: 7-bit noise | Backspace: isolate noise");
            ui.label("Two pulse voices, wave, noise and both PCM FIFOs play together. Enable audio to listen.");
        }
        if self.rom_name == "wave.gba" {
            ui.label("Z: effect + PCM | X: bank 1 | Backspace: 64 digits | Up: 75% | Down: mute | Release: restart");
            ui.label("Pulse accompaniment left; wave effect right; PCM both sides. Enable audio to listen.");
        }
        if self.rom_name == "pulse.gba" {
            let lead = self.session.inspect16(0x0300000a).unwrap_or(0);
            let second = self.session.inspect16(0x0300000c).unwrap_or(0);
            ui.label(format!(
                "Pulse source notes: lead {lead}, second {second}; square color follows each note"
            ));
            ui.label("Z: PCM | X: sweep | Backspace: duty | Up: envelope | Down: mute | Release: restart");
        }
        if self.rom_name == "pcm.gba" {
            ui.label("Mixer while holding Z: Left/Right = route, Down = sound off, Backspace = half volume, Up = bias/PWM, X = FIFO reset");
        }
        egui::ComboBox::from_label("Backup override for next ROM load")
            .selected_text(
                self.backup_override
                    .map_or_else(|| "Auto detect".to_owned(), |kind| format!("{kind:?}")),
            )
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.backup_override, None, "Auto detect");
                for kind in [
                    gba_session::BackupType::None,
                    gba_session::BackupType::Sram,
                    gba_session::BackupType::Eeprom,
                    gba_session::BackupType::Eeprom512,
                    gba_session::BackupType::Eeprom8k,
                    gba_session::BackupType::Flash64,
                    gba_session::BackupType::Flash128,
                ] {
                    ui.selectable_value(&mut self.backup_override, Some(kind), format!("{kind:?}"));
                }
            });
        let backup = self.session.backup_selection();
        ui.label(format!(
            "Backup: {:?} | detected: {:?} | override: {:?}",
            backup.selected(),
            backup.detection,
            backup.manual_override
        ));
        if matches!(
            backup.selected(),
            Some(
                gba_session::BackupType::Eeprom
                    | gba_session::BackupType::Eeprom512
                    | gba_session::BackupType::Eeprom8k
            )
        ) {
            let size = self.session.save_status().map_or(0, |status| status.len);
            ui.label(if size == 0 {
                "EEPROM capacity unresolved; serial commands or a validated backup resolve it"
                    .to_owned()
            } else {
                format!("EEPROM capacity: {size} bytes")
            });
        }
        if backup.selected().is_none() {
            ui.label("Backup unresolved: choose an override and reload the cartridge if save hardware is needed.");
        }
        ui.label(&self.status);
        if self.rom_name == "calculations.gba" {
            match (
                self.session.inspect16(0x03000000),
                self.session.inspect16(0x03000002),
                self.session.inspect16(0x03000004),
            ) {
                (Ok(0x005d), Ok(result), Ok(first_failure)) => {
                    ui.label(format!(
                        "CPU diagnostic complete: mailbox 0x005d | result {result} | first failing case {first_failure}"
                    ));
                }
                _ => {
                    ui.label("CPU diagnostic running");
                }
            }
        }
        if matches!(self.rom_name.as_str(), "keypad-or.gba" | "keypad-and.gba")
            && let (Ok(callbacks), Ok(before), Ok(after), Ok(wakes)) = (
                self.session.inspect16(0x03000002),
                self.session.inspect16(0x03000004),
                self.session.inspect16(0x03000006),
                self.session.inspect16(0x03000008),
            )
        {
            ui.label(format!("Keypad callbacks: {callbacks} | wakes: {wakes} | IF before: {before:#06x} | after: {after:#06x}"));
        }
        ui.label("Click Load ROM or drop a .gba file here.");
        ui.label("Demo: arrow keys move the square once per GBA frame.");
        ui.collapsing("Performance", |ui| {
            ui.label(self.core_times.label("Core execution"));
            ui.label(self.conversion_times.label("Pixel conversion"));
            ui.label(self.upload_times.label("Texture submission"));

            let pacing = if self.session.slowed() {
                egui::RichText::new("Pacing: work budget reached").color(egui::Color32::YELLOW)
            } else {
                egui::RichText::new("Pacing: real-time")
            };

            ui.label(pacing);

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
            let response = ui.add(
                egui::Image::from_texture(texture)
                    .fit_to_exact_size(size)
                    .sense(egui::Sense::click()),
            );
            self.game_rect = Some(response.rect);
            if response.clicked() {
                ui.ctx().memory_mut(|memory| {
                    if let Some(id) = memory.focused() {
                        memory.surrender_focus(id);
                    }
                });
            }
        }
    }
}

impl eframe::App for GbaApp {
    fn raw_input_hook(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        self.prepare_host_input(ctx, raw);
    }

    /// Executes bounded guest work while running and schedules the next host wake.
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.capture.is_none()
            && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::F1))
        {
            self.debug_ui = !self.debug_ui;
            ctx.request_repaint();
        }
        #[cfg(not(target_arch = "wasm32"))]
        if ctx.input(|input| input.viewport().close_requested())
            && (self.storage.busy || self.restoring_save || self.session.persistence_dirty())
        {
            // Ordinary native close is a save barrier. Failure keeps the window
            // open with pending bytes available for retry or export.
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.close_when_saved = true;
            if !self.session.paused() {
                self.session.toggle_pause();
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        if self.capture_requested {
            let image = ctx.input(|input| {
                input.events.iter().find_map(|event| {
                    if let egui::Event::Screenshot { image, .. } = event {
                        Some(Arc::clone(image))
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
        let dropped = ctx.input_mut(|input| {
            let file = input.raw.dropped_files.pop();

            // The existing behavior intentionally processes only
            // the final file from one drop operation.
            input.raw.dropped_files.clear();

            file
        });
        if let Some(file) = dropped {
            let name = file
                .path()
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            #[cfg(not(target_arch = "wasm32"))]
            match file.bytes() {
                Ok(bytes) => self.load_rom_bytes(&name, &bytes),
                Err(error) => {
                    self.report_error(NoticeKind::Rom, format!("ROM read failed: {error}"))
                }
            }
            #[cfg(target_arch = "wasm32")]
            {
                let backup_override = self.backup_override.take();
                self.load_generation = self.load_generation.wrapping_add(1);
                self.picker_open = true;
                self.bios_picker_open = false;
                self.pending_bios = None;
                let generation = self.load_generation;
                let sender = self.rom_sender.clone();
                let ctx = ctx.clone();
                self.status = format!("Reading {name}");
                wasm_bindgen_futures::spawn_local(async move {
                    let result = Some((name, file.bytes_async().await));
                    let _ = sender.send(RomRead {
                        generation,
                        backup_override,
                        result,
                    });
                    ctx.request_repaint();
                });
            }
        }
        while let Ok(completion) = self.bios_receiver.try_recv() {
            if completion.generation != self.load_generation {
                continue;
            }
            self.bios_picker_open = false;
            match completion.result {
                Some((name, Ok(bytes))) => self.load_bios_request(&name, &bytes),
                Some((_, Err(error))) => {
                    self.report_error(NoticeKind::Bios, format!("BIOS read failed: {error}"))
                }
                None => {} // Cancellation preserves installed firmware and explicit pause.
            }
        }

        while let Ok(completion) = self.rom_receiver.try_recv() {
            if completion.generation != self.load_generation {
                continue;
            }
            self.picker_open = false;
            let backup_override = completion.backup_override;
            if let Some((name, result)) = completion.result {
                match result {
                    Ok(bytes) => self.load_rom_request(&name, &bytes, backup_override),
                    Err(error) => {
                        self.restore_backup_override(backup_override);
                        self.report_error(NoticeKind::Rom, format!("ROM read failed: {error}"));
                    }
                }
            } else {
                self.restore_backup_override(backup_override);
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        let unix_seconds = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        #[cfg(target_arch = "wasm32")]
        let unix_seconds = (js_sys::Date::now() / 1000.0).max(0.0) as u64;
        self.session.set_rtc_time(unix_seconds);
        self.poll_save_storage(ctx);
        #[cfg(not(target_arch = "wasm32"))]
        if self.close_when_saved
            && !self.storage.busy
            && !self.restoring_save
            && !self.session.persistence_dirty()
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        let focused = self.host_focused;
        let visible = self.host_visible;
        self.poll_binding_capture(ctx);
        let running = self.sync_execution(focused, visible);
        if self.replay_deadline.is_some() && !(focused && visible) {
            self.replay_deadline = None;
            self.status =
                "Replay cancelled on focus loss; run it again to compare positions".into();
        }
        if running
            && self.replay_deadline.is_none()
            && self.input_owner == InputOwner::Gameplay
            && !self.keyboard_ui_owned(ctx)
        {
            self.poll_keyboard(ctx);
        } else if self.replay_deadline.is_none() {
            self.own_ui_keys(ctx);
        }
        let start = Instant::now();
        let now = self.host_origin.elapsed();
        let result = if let Some(deadline) = self.replay_deadline {
            self.session.advance_host_time_until(now, deadline)
        } else {
            self.session.advance_host_time(now)
        };
        match result {
            Ok(Some(_)) => {
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
            Ok(None) => {}
            Err(error) => {
                self.audio.set_playing(false);
                self.report_error(
                    NoticeKind::Emulation,
                    format!("Guest execution failed: {error}"),
                );
                self.execution_faulted = true;
                self.sync_execution(focused, visible);
                return;
            }
        }
        self.audio.pump(&mut self.session);
        if self
            .replay_deadline
            .is_some_and(|deadline| self.session.cycles() >= deadline)
        {
            self.replay_deadline = None;
            self.user_paused = true;
            self.sync_execution(focused, visible);
            self.status = if self.rom_name == "pcm.gba" {
                match (
                    self.session.inspect16(0x03000000),
                    self.session.inspect16(0x03000004),
                    self.session.inspect16(0x03000006),
                ) {
                    (Ok(0x66), Ok(0), Ok(2)) => {
                        "PCM replay complete: two transitions; square red and sound idle".to_owned()
                    }
                    _ => "PCM replay failed: mailbox mismatch".to_owned(),
                }
            } else if matches!(self.rom_name.as_str(), "keypad-or.gba" | "keypad-and.gba") {
                // Keypad mailbox fields are IRQ evidence, never square coordinates.
                // Report success only after validating the complete guest contract.
                match (
                    self.session.inspect16(0x03000000),
                    self.session.inspect16(0x03000002),
                    self.session.inspect16(0x03000008),
                    self.session.inspect16(0x03000004),
                    self.session.inspect16(0x03000006),
                ) {
                    (Ok(0x0064), Ok(1), Ok(1), Ok(0x1000), Ok(0)) => format!(
                        "Keypad replay complete: {} | one callback and wake | IF 0x1000 acknowledged to 0",
                        self.rom_name
                    ),
                    _ => format!(
                        "Keypad replay failed: {} completion mailbox does not match the expected IRQ result",
                        self.rom_name
                    ),
                }
            } else if self.rom_name == "dma.gba" {
                match (
                    self.session.inspect16(0x03000000),
                    self.session.inspect16(0x03000002),
                    self.session.inspect16(0x03000004),
                    self.session.inspect16(0x03000006),
                    self.session.inspect16(0x03000008),
                ) {
                    (Ok(0x65), Ok(9), Ok(8), Ok(0x801), Ok(0)) =>
                        "DMA replay complete: 9 VBlanks, 8 uploads; IF acknowledged; final frame red".to_owned(),
                    _ => "DMA replay failed: completion mailbox mismatch".to_owned(),
                }
            } else {
                match (
                    self.session.inspect16(0x03000002),
                    self.session.inspect16(0x03000004),
                ) {
                    (Ok(x), Ok(y))
                        if matches!(self.rom_name.as_str(), "sprites.gba" | "irq-sprites.gba") =>
                    {
                        match (
                            self.session.inspect16(0x03000008),
                            self.session.inspect16(0x0300000a),
                        ) {
                            (Ok(player_x), Ok(player_y)) => format!(
                                "Replay complete: player ({player_x}, {player_y}), scroll ({x}, {y}); expected (116, 74), (2, 0)"
                            ),
                            _ => "Replay complete; player position unavailable".to_owned(),
                        }
                    }
                    (Ok(x), Ok(y)) if self.rom_name == "tiled.gba" => {
                        format!("Replay complete: scroll ({x}, {y}); expected (2, 0)")
                    }
                    (Ok(x), Ok(y)) => {
                        format!("Replay complete: square at ({x}, {y}); expected (114, 73)")
                    }
                    _ => "Replay complete; guest position unavailable".to_owned(),
                }
            };
        }
        if self.execution_allowed() {
            // This wake is a presentation request. Session elapsed-time pacing,
            // rather than callback count, determines how many GBA cycles execute.
            ctx.request_repaint();
        }
    }

    /// Uses eframe's autosave/shutdown lifecycle; gameplay never serializes settings.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.settings
            .save(storage, self.audio.volume, self.audio.muted);
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

/// Fits the framebuffer continuously to the viewport while preserving 3:2.
/// Nearest-neighbor filtering remains the texture upload policy.
fn player_screen_size(available: egui::Vec2) -> egui::Vec2 {
    fitted_screen_size(available)
}

/// Display sizing is host presentation only. Integer multipliers count physical
/// pixels; fractional Fit remains usable when a viewport cannot contain 1x.
struct ScreenLayout {
    size: egui::Vec2,
    integer_fallback: bool,
}

fn player_screen_layout(
    available: egui::Vec2,
    scaling: crate::settings::Scaling,
    pixels_per_point: f32,
) -> ScreenLayout {
    let integer = scaling == crate::settings::Scaling::Integer;
    let multiplier = ((available.x * pixels_per_point / WIDTH as f32)
        .min(available.y * pixels_per_point / HEIGHT as f32))
    .floor();
    let integer_fallback = integer && multiplier < 1.0;
    let size = if integer && !integer_fallback {
        egui::vec2(WIDTH as f32, HEIGHT as f32) * (multiplier / pixels_per_point)
    } else {
        player_screen_size(available)
    };
    ScreenLayout {
        size,
        integer_fallback,
    }
}

/// Resizing must fill the available area without a scale cap or aspect distortion.
#[cfg(test)]
mod tests {
    use super::*;

    // Export is a recovery copy, never acknowledgement of a failed automatic
    // write. Its completion must not replace the persistent underlying reason.
    #[test]
    fn export_keeps_the_failed_write_reason_and_dirty_revision() {
        let mut app = GbaApp::create(false);
        app.load_rom_bytes("sram.gba", SRAM_ROM);
        app.restoring_save = false;
        app.sync_execution(true, true);
        app.session.advance_frame().unwrap();
        assert!(app.session.persistence_dirty());
        let completion = |outcome| crate::saves::Completion {
            identity: app.save_identity.unwrap(),
            generation: app.save_generation,
            revision: 0,
            rtc_revision: None,
            outcome,
        };
        let failed = completion(crate::saves::Outcome::Written(Err("disk full".into())));
        let exported = completion(crate::saves::Outcome::Exported(Ok(true)));
        app.apply_save_completion(failed);
        let dirty = app.session.persistence_dirty();
        app.apply_save_completion(exported);
        assert!(app.storage.failed);
        assert_eq!(app.session.persistence_dirty(), dirty);
        assert!(
            app.save_failure
                .as_ref()
                .is_some_and(|failure| failure.message.contains("disk full"))
        );
    }

    // A rejected file read/format never reaches session.import_save. Previously
    // its reason appeared only in debug storage text, leaving normal players blind.
    #[test]
    fn rejected_import_is_visible_without_changing_cartridge_data() {
        let mut app = GbaApp::create(false);
        app.load_rom_bytes("sram.gba", SRAM_ROM);
        app.restoring_save = false;
        app.save_import_open = true;
        let before = app.session.save_image().unwrap().bytes;
        app.apply_save_completion(crate::saves::Completion {
            identity: app.save_identity.unwrap(),
            generation: app.save_generation,
            revision: 0,
            rtc_revision: None,
            outcome: crate::saves::Outcome::Imported(Err("ROM identity mismatch".into())),
        });
        assert!(!app.save_import_open);
        assert_eq!(app.session.save_image().unwrap().bytes, before);
        assert_eq!(
            app.notice.as_ref().map(|notice| notice.message.as_str()),
            Some("Import rejected: ROM identity mismatch")
        );
    }

    // Restoring SRAM exposes initialized bytes, not a validated stored save. An
    // export here would present blank data as recovery after a failed storage read.
    #[test]
    fn unresolved_restore_cannot_export_initialized_backup() {
        let mut app = GbaApp::create(false);
        app.load_rom_bytes("sram.gba", SRAM_ROM);
        app.storage.failed = true;
        assert!(app.restoring_save);
        assert!(
            app.can_import_save(),
            "Validated import remains a recovery path"
        );
        assert!(
            !app.can_export_save(),
            "Export offered unvalidated blank backup"
        );
    }

    // Fit-only tests miss fractional host scale: Integer must count physical
    // pixels, fall back below 1x, and recover without changing the preference.
    #[test]
    fn integer_display_uses_physical_pixels_and_recovers_after_small_viewport() {
        use crate::settings::Scaling;
        let large = player_screen_layout(egui::vec2(500.0, 340.0), Scaling::Integer, 1.5);
        assert_eq!(large.size, egui::vec2(480.0, 320.0));
        assert!(!large.integer_fallback);
        let small = player_screen_layout(egui::vec2(100.0, 80.0), Scaling::Integer, 1.5);
        assert_eq!(small.size, egui::vec2(100.0, 100.0 * 2.0 / 3.0));
        assert!(small.integer_fallback);
        assert!(
            !player_screen_layout(egui::vec2(500.0, 340.0), Scaling::Integer, 2.0).integer_fallback
        );
        let fit = player_screen_layout(egui::vec2(500.0, 340.0), Scaling::Fit, 1.5);
        assert_eq!(fit.size, player_screen_size(egui::vec2(500.0, 340.0)));
    }

    // A valid cartridge selected before firmware must survive until BIOS startup.
    #[test]
    fn rom_first_waits_for_bios_and_boots_the_retained_request() {
        let mut app = GbaApp::create(false);
        let mut rom = BUTTONS_ROM.to_vec();
        rom.push(0); // Distinct from controlled fixture identity: use retail startup.
        app.load_rom_bytes("pending.gba", &rom);
        assert!(!app.loaded);
        assert!(app.pending_rom.is_some(), "ROM-first request was discarded");
        app.load_bios_bytes(&vec![0; 16384]);
        app.poll_save_storage(&egui::Context::default());
        assert!(app.loaded);
        assert_eq!(app.rom_name, "pending.gba");
        assert!(app.pending_rom.is_none());
    }

    // Reset must not resume a game the user explicitly paused.
    #[test]
    fn reset_retains_the_users_pause_choice() {
        let mut app = GbaApp::create(false);
        app.load_rom_bytes("buttons.gba", BUTTONS_ROM);
        app.toggle_user_pause();
        app.reset_demo();
        assert!(app.session.paused(), "Reset unexpectedly resumed gameplay");
    }

    // Session focus tests do not cover application windows opening after logic.
    // Settings must release submitted input immediately and retain pause intent.
    #[test]
    fn settings_release_input_and_preserve_explicit_pause() {
        let mut app = GbaApp::create(false);
        app.load_rom_bytes("buttons.gba", BUTTONS_ROM);
        app.session.set_button(Button::Right, true);
        app.settings_open = true;
        assert!(!app.sync_execution(true, true));
        assert!(!app.session.button_pressed(Button::Right));
        app.settings_open = false;
        assert!(app.sync_execution(true, true));
        app.toggle_user_pause();
        app.settings_open = true;
        app.sync_execution(true, true);
        app.settings_open = false;
        assert!(!app.sync_execution(true, true));
        assert!(app.user_paused);
        assert!(app.session.paused());
    }

    // A captured Enter must not activate a focused application action in the
    // same frame, and a captured gameplay key must require release before play.
    // Pure mapping tests cannot catch this app/egui ownership boundary.
    #[test]
    fn captured_keys_cannot_activate_ui_or_press_gameplay_on_return() {
        fn frame(
            ctx: &egui::Context,
            key: egui::Key,
            pressed: bool,
            draw: impl FnMut(&mut egui::Ui),
        ) {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    events: vec![egui::Event::Key {
                        key,
                        physical_key: None,
                        pressed,
                        repeat: false,
                        modifiers: egui::Modifiers::NONE,
                    }],
                    focused: true,
                    ..Default::default()
                },
                draw,
            );
            output.textures_delta.clear();
        }
        let ctx = egui::Context::default();
        let mut app = GbaApp::create(false);
        app.load_rom_bytes("buttons.gba", BUTTONS_ROM);
        app.settings_open = true;
        app.session.set_button(Button::A, true);
        frame(&ctx, egui::Key::Enter, true, |ui| {
            app.begin_binding_capture(ui.ctx(), 0);
            assert!(!app.session.button_pressed(Button::A));
        });
        frame(&ctx, egui::Key::Enter, false, |ui| {
            app.poll_binding_capture(ui.ctx());
            let response = ui.button("Close settings");
            response.request_focus();
        });
        frame(&ctx, egui::Key::Enter, true, |ui| {
            app.poll_binding_capture(ui.ctx());
            assert!(
                !ui.button("Close settings").clicked(),
                "Captured Enter activated UI"
            );
        });
        assert_eq!(app.settings.preferences.bindings[0], egui::Key::Enter);
        assert_eq!(app.settings.preferences.bindings[4], egui::Key::Z);
        assert_eq!(app.binding_labels[0].full, "Enter");
        app.settings_open = false;
        app.sync_execution(true, true);
        app.poll_keyboard(&ctx);
        assert!(!app.session.button_pressed(Button::A));
        frame(&ctx, egui::Key::Enter, false, |ui| {
            app.poll_keyboard(ui.ctx())
        });
        frame(&ctx, egui::Key::Enter, true, |ui| {
            app.poll_keyboard(ui.ctx())
        });
        assert!(app.session.button_pressed(Button::A));
        assert!(!app.session.button_pressed(Button::Start));
    }

    // Session release alone cannot stop app polling from resubmitting a held key
    // after focus loss. This tests the host/session boundary, not core KEYINPUT.
    #[test]
    fn held_key_requires_release_after_focus_loss_and_reset() {
        let mut app = GbaApp::create(false);
        app.load_rom_bytes("buttons.gba", BUTTONS_ROM);
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(
            egui::RawInput {
                events: vec![egui::Event::Key {
                    key: egui::Key::ArrowRight,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
                ..Default::default()
            },
            |ui| {
                app.poll_keyboard(ui.ctx());
                assert!(app.session.button_pressed(Button::Right));
                app.sync_execution(false, true);
                app.sync_execution(true, true);
                app.poll_keyboard(ui.ctx());
                assert!(
                    !app.session.button_pressed(Button::Right),
                    "Held key returned after focus loss"
                );
            },
        );
        output.textures_delta.clear();
        for pressed in [false, true] {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    events: vec![egui::Event::Key {
                        key: egui::Key::ArrowRight,
                        physical_key: None,
                        pressed,
                        repeat: false,
                        modifiers: egui::Modifiers::NONE,
                    }],
                    ..Default::default()
                },
                |ui| app.poll_keyboard(ui.ctx()),
            );
            output.textures_delta.clear();
        }
        assert!(app.session.button_pressed(Button::Right));
        app.reset_demo();
        app.poll_keyboard(&ctx);
        assert!(
            !app.session.button_pressed(Button::Right),
            "Reset restored held input"
        );
    }

    // A completion/reset must not assume visibility or focus and reactivate a
    // background machine. Session-only lifecycle tests do not exercise app actions.
    #[test]
    fn hidden_reset_and_replacement_completion_do_not_resume_execution() {
        let mut app = GbaApp::create(false);
        app.load_rom_bytes("buttons.gba", BUTTONS_ROM);
        app.sync_execution(false, false);
        app.reset_demo();
        app.session.advance_host_time(Duration::ZERO).unwrap();
        app.session
            .advance_host_time(Duration::from_secs(100))
            .unwrap();
        assert_eq!(
            app.session.cycles(),
            Cycle(0),
            "Reset resumed hidden execution"
        );
        app.storage.busy = true;
        app.load_bios_bytes(&vec![0; 16384]);
        assert!(app.pending_bios.is_some());
        assert!(!app.can_reset());
        app.storage.busy = false;
        app.poll_save_storage(&egui::Context::default());
        app.session.advance_host_time(Duration::ZERO).unwrap();
        app.session
            .advance_host_time(Duration::from_secs(200))
            .unwrap();
        assert_eq!(
            app.session.cycles(),
            Cycle(0),
            "Replacement completion resumed hidden execution"
        );
        app.settings_open = true;
        app.sync_execution(true, true);
        assert!(!app.sync_execution(true, true));
        app.settings_open = false;
        assert!(app.sync_execution(true, true));
        app.session
            .advance_host_time(Duration::from_secs(300))
            .unwrap();
        assert_eq!(
            app.session.cycles(),
            Cycle(0),
            "Background time became catch-up debt"
        );
    }

    // A remapped navigation key must act on the guest instead of moving focus to
    // application controls. Removing events after egui starts the pass is too late.
    #[test]
    fn gameplay_tab_does_not_navigate_application_widgets() {
        let mut app = GbaApp::create(false);
        app.load_rom_bytes("buttons.gba", BUTTONS_ROM);
        app.settings.preferences.bindings[0] = egui::Key::Tab;
        let ctx = egui::Context::default();
        for _ in 0..2 {
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                let _ = ui.button("Pause");
                let _ = ui.button("Settings");
            });
            output.textures_delta.clear();
        }
        let mut raw = egui::RawInput {
            focused: true,
            events: vec![egui::Event::Key {
                key: egui::Key::Tab,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            ..Default::default()
        };
        eframe::App::raw_input_hook(&mut app, &ctx, &mut raw);
        let mut output = ctx.run_ui(raw, |ui| {
            app.poll_keyboard(ui.ctx());
            let _ = ui.button("Pause");
            let _ = ui.button("Settings");
        });
        output.textures_delta.clear();
        assert!(app.session.button_pressed(Button::A));
        assert!(
            ctx.memory(|memory| memory.focused().is_none()),
            "Gameplay Tab moved application focus"
        );
    }

    // Gameplay presses are intentionally absent from egui's keys_down. Capture
    // must also wait for our host-held map, or a repeated held key becomes a binding.
    #[test]
    fn capture_waits_for_keys_held_before_gameplay_event_filtering() {
        let mut app = GbaApp::create(false);
        app.load_rom_bytes("buttons.gba", BUTTONS_ROM);
        let ctx = egui::Context::default();
        for (index, pressed) in [true, true, false].into_iter().enumerate() {
            let mut raw = egui::RawInput {
                focused: true,
                events: vec![egui::Event::Key {
                    key: egui::Key::ArrowRight,
                    physical_key: None,
                    pressed,
                    repeat: index == 1,
                    modifiers: egui::Modifiers::NONE,
                }],
                ..Default::default()
            };
            eframe::App::raw_input_hook(&mut app, &ctx, &mut raw);
            let mut output = ctx.run_ui(raw, |ui| {
                if index == 0 {
                    app.poll_keyboard(ui.ctx());
                    app.settings_open = true;
                    app.begin_binding_capture(ui.ctx(), 0);
                } else {
                    app.poll_binding_capture(ui.ctx());
                }
            });
            output.textures_delta.clear();
            assert!(
                app.capture.is_some(),
                "A previously held gameplay key completed capture"
            );
            assert_eq!(app.settings.preferences.bindings[0], egui::Key::Z);
        }
    }

    // Previous-frame UI ownership must not suppress a genuinely new press after
    // Settings closes. Held-key tests do not cover this separate transition gap.
    #[test]
    fn fresh_key_after_settings_close_is_not_suppressed_by_stale_ui_ownership() {
        let mut app = GbaApp::create(false);
        app.load_rom_bytes("buttons.gba", BUTTONS_ROM);
        app.settings_open = true;
        let ctx = egui::Context::default();
        for key in [None, Some(egui::Key::Escape), Some(egui::Key::ArrowRight)] {
            let mut raw = egui::RawInput {
                focused: true,
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(760.0, 540.0),
                )),
                events: key
                    .map(|key| {
                        vec![egui::Event::Key {
                            key,
                            physical_key: None,
                            pressed: true,
                            repeat: false,
                            modifiers: egui::Modifiers::NONE,
                        }]
                    })
                    .unwrap_or_default(),
                ..Default::default()
            };
            eframe::App::raw_input_hook(&mut app, &ctx, &mut raw);
            let mut output = ctx.run_ui(raw, |ui| {
                app.poll_keyboard(ui.ctx());
                app.draw_ui(ui);
            });
            output.textures_delta.clear();
        }
        assert!(!app.settings_open);
        assert!(
            app.session.button_pressed(Button::Right),
            "A fresh press after Settings was discarded"
        );
    }

    // Capture owns Tab before egui resolves focus movement. Consuming it later
    // prevents button clicks but still leaks navigation into Settings widgets.
    #[test]
    fn captured_tab_does_not_navigate_application_widgets() {
        let mut app = GbaApp::create(false);
        app.load_rom_bytes("buttons.gba", BUTTONS_ROM);
        app.settings_open = true;
        let ctx = egui::Context::default();
        for _ in 0..2 {
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                let _ = ui.button("Change");
                let _ = ui.button("Close settings");
            });
            output.textures_delta.clear();
        }
        app.capture = Some(Capture::new(0, true));
        let mut raw = egui::RawInput {
            focused: true,
            events: vec![egui::Event::Key {
                key: egui::Key::Tab,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            ..Default::default()
        };
        eframe::App::raw_input_hook(&mut app, &ctx, &mut raw);
        let mut output = ctx.run_ui(raw, |ui| {
            app.poll_binding_capture(ui.ctx());
            let _ = ui.button("Change");
            let _ = ui.button("Close settings");
        });
        output.textures_delta.clear();
        assert_eq!(app.settings.preferences.bindings[0], egui::Key::Tab);
        assert!(
            ctx.memory(|memory| memory.focused().is_none()),
            "Captured Tab navigated Settings"
        );
    }

    // Picker cancellation tests cannot catch firmware being discarded while a
    // previously issued cartridge write is still outstanding.
    #[test]
    fn bios_replacement_commits_only_after_the_save_barrier() {
        let mut app = GbaApp::create(false);
        app.load_rom_bytes("buttons.gba", BUTTONS_ROM);
        app.load_bios_bytes(&vec![0; 16384]);
        let previous_hash = app.bios_hash.clone();
        app.storage.busy = true;
        let replacement = vec![1; 16384];
        app.load_bios_bytes(&replacement);
        assert_eq!(app.bios_hash, previous_hash);
        app.storage.busy = false;
        app.poll_save_storage(&egui::Context::default());
        assert_eq!(
            app.bios_hash,
            Some(format!("{:x}", Sha256::digest(&replacement)))
        );
    }

    #[test]
    fn player_screen_fills_resized_viewports_without_distorting_aspect_ratio() {
        assert_eq!(
            player_screen_size(egui::vec2(960.0, 640.0)),
            egui::vec2(960.0, 640.0)
        );
        assert_eq!(
            player_screen_size(egui::vec2(600.0, 410.0)),
            egui::vec2(600.0, 400.0)
        );
        assert_eq!(
            player_screen_size(egui::vec2(240.0, 160.0)),
            egui::vec2(240.0, 160.0)
        );
        assert_eq!(
            player_screen_size(egui::vec2(120.0, 100.0)),
            egui::vec2(120.0, 80.0)
        );
        assert_eq!(
            player_screen_size(egui::vec2(300.0, 80.0)),
            egui::vec2(120.0, 80.0)
        );
        assert_eq!(player_screen_size(egui::Vec2::ZERO), egui::Vec2::ZERO);
    }
}

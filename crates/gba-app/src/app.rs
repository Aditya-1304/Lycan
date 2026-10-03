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

/// Original score cartridges use the normal loader and persistence route.
const FLASH_DIAGNOSTIC_ROM: &[u8] = include_bytes!("../../../roms/gba-tests/save/flash64.gba");
const BANKED_DIAGNOSTIC_ROM: &[u8] = include_bytes!("../../../roms/gba-tests/save/flash128.gba");
const EEPROM512_ROM: &[u8] = include_bytes!("../../../roms/eeprom512-score.gba");
const EEPROM8K_ROM: &[u8] = include_bytes!("../../../roms/eeprom8k-score.gba");
const BANKED_ROM: &[u8] = include_bytes!("../../../roms/flash-banked-score.gba");
const FLASH_ROM: &[u8] = include_bytes!("../../../roms/flash-score.gba");
const RTC_ROM: &[u8] = include_bytes!("../../../roms/rtc.gba");
const SRAM_ROM: &[u8] = include_bytes!("../../../roms/sram.gba");

/// Original display diagnostics use controlled ARM startup; retail ROMs retain BIOS boot.
const DISPLAY_DIAGNOSTIC_ROMS: [&[u8]; 9] = [
    include_bytes!("../../../roms/raster.gba"),
    include_bytes!("../../../roms/blend.gba"),
    include_bytes!("../../../roms/window.gba"),
    include_bytes!("../../../roms/affine-object.gba"),
    include_bytes!("../../../roms/affine-mode-1.gba"),
    include_bytes!("../../../roms/affine-mode-2.gba"),
    include_bytes!("../../../roms/affine-mode-3.gba"),
    include_bytes!("../../../roms/affine-mode-4.gba"),
    include_bytes!("../../../roms/affine-mode-5.gba"),
];

const WIDTH: usize = SCREEN_WIDTH;
const HEIGHT: usize = SCREEN_HEIGHT;

const BUTTONS_ROM: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../roms/buttons.gba"
));

/// Loading this artifact uses the same session,
/// master-clock pacing, and completed-frame presentation as dropped ROMs.
const PALETTE_ROM: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../roms/palette.gba"
));

/// Guest calculations use the shared session and timed display route on both
/// platforms; the application only observes the guest's completion mailbox.
const CALCULATIONS_ROM: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../roms/calculations.gba"
));

/// The bitmap-copy guest uses the shared loader and frame pacing on native and
/// browser hosts; copying, redraw, and return all execute inside the emulated CPU.
const COPY_ROM: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../roms/copy.gba"));

/// ARM draws the bar after a Thumb routine updates its guest-owned counter.
const COUNTER_ROM: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../roms/counter.gba"
));

/// Pinned upstream diagnostic bytes also identify the explicit test-firmware
/// loading route, including when the same ROM is opened from a file dialog.
const ARM_DIAGNOSTIC_ROM: &[u8] = include_bytes!("../../../roms/gba-tests/arm/arm.gba");
const THUMB_DIAGNOSTIC_ROM: &[u8] = include_bytes!("../../../roms/gba-tests/thumb/thumb.gba");

/// Original mode-0 scene with guest-owned scrolling and the pinned reference ROM.
const TILED_ROM: &[u8] = include_bytes!("../../../roms/tiled.gba");
const STRIPES_ROM: &[u8] = include_bytes!("../../../roms/gba-tests/stripes.gba");
const TILED_INPUT: &[(Cycle, Button, bool)] = &include!("../../../roms/tiled/input.rs");

/// Explicit original IRQ diagnostic; normal cartridges never map test firmware.
const KEYPAD_OR_ROM: &[u8] = include_bytes!("../../../roms/keypad-or.gba");
const KEYPAD_AND_ROM: &[u8] = include_bytes!("../../../roms/keypad-and.gba");
/// The same logical input deadlines are used by the bounded headless runner.
const KEYPAD_INPUT: &[(Cycle, Button, bool)] = &include!("../../../roms/keypad/input.rs");
const VBLANK_ROM: &[u8] = include_bytes!("../../../roms/vblank.gba");
/// Original paired noise events with all PSG voices and both Direct Sound FIFOs.
const NOISE_ROM: &[u8] = include_bytes!("../../../roms/noise.gba");
/// Original wave effect scene mixed with pulse voices and Direct Sound.
const WAVE_ROM: &[u8] = include_bytes!("../../../roms/wave.gba");
/// Original two-voice pulse scene, using the same explicit firmware as PCM.
const PULSE_ROM: &[u8] = include_bytes!("../../../roms/pulse.gba");
const PCM_ROM: &[u8] = include_bytes!("../../../roms/pcm.gba");
const PCM_INPUT: &[(Cycle, Button, bool)] = &include!("../../../roms/pcm/input.rs");

const DMA_ROM: &[u8] = include_bytes!("../../../roms/dma.gba");
const DMA_INPUT: &[(Cycle, Button, bool)] = &include!("../../../roms/dma/input.rs");
/// IRQ variant shares the polling scene assets and scripted logical input.
const IRQ_SPRITES_ROM: &[u8] = include_bytes!("../../../roms/irq-sprites.gba");
/// Original guest-owned sprite scene and its independently verified replay.
const SPRITES_ROM: &[u8] = include_bytes!("../../../roms/sprites.gba");
const SPRITE_INPUT: &[(Cycle, Button, bool)] = &include!("../../../roms/sprites/input.rs");
const MEMORY_ROM: &[u8] = include_bytes!("../../../roms/gba-tests/memory/memory.gba");

/// The replay uses the same ordered cycle transitions verified by gba-tools.
const DEMO_INPUT: &[(Cycle, Button, bool)] = &include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../roms/buttons/input.rs"
));

use crate::input::{
    BINDING_BUTTONS, BindingLabel, Capture, CaptureResult, binding_labels, button_name,
};

/// Shared native/browser application displaying pixels produced by guest execution.
pub struct GbaApp {
    /// Presentation mode only; switching modes preserves the active session.
    debug_ui: bool,
    settings: crate::settings::Settings,
    /// A settings window temporarily owns input without changing explicit pause.
    settings_open: bool,
    /// Transient capture state never reaches persisted preferences.
    capture: Option<Capture>,
    capture_notice: Option<String>,
    /// Formatting is cached at load/edit boundaries, not during gameplay drawing.
    binding_labels: [BindingLabel; 10],
    /// Menus gate logical input; settings additionally suspend execution.
    ui_keys_owned: bool,
    /// Still-held keys remain ineligible after a UI interaction until released.
    suppressed_keys: [bool; 10],
    notice: Option<String>,
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
        Self::create_with_settings(debug_ui, crate::settings::Settings::load(cc.storage))
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
            capture: None,
            capture_notice: None,
            ui_keys_owned: false,
            suppressed_keys: [false; 10],
            notice: None,
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
        self.save_identity.is_some()
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
            && !self.storage.busy
            && self
                .session
                .save_status()
                .is_some_and(|status| status.len > 0)
    }

    fn report_error(&mut self, message: String) {
        self.status.clone_from(&message);
        self.notice = Some(message);
    }

    /// Invalidate already submitted keys when UI takes ownership after logic.
    /// The fixed suppression array avoids event collections or per-frame allocation.
    fn own_ui_keys(&mut self, ctx: &egui::Context) {
        ctx.input(|input| {
            for (blocked, key) in self
                .suppressed_keys
                .iter_mut()
                .zip(self.settings.preferences.bindings)
            {
                *blocked |= input.key_down(key);
            }
        });
        self.session.release_all_buttons();
    }

    /// Capture starts from a UI-owned frame and immediately invalidates logical
    /// input submitted by earlier logic. Activation events are not processed here.
    fn begin_binding_capture(&mut self, ctx: &egui::Context, slot: usize) {
        self.own_ui_keys(ctx);
        let released = ctx.input(|input| input.keys_down.is_empty() && !input.pointer.any_down());
        self.capture = Some(Capture::new(slot, released));
        self.capture_notice = None;
    }

    /// Rebuild presentation and suppression together after an atomic edit. Slots
    /// can exchange host keys, so suppression must be resampled for the new map.
    fn bindings_changed(&mut self, ctx: &egui::Context) {
        self.binding_labels = binding_labels(&self.settings.preferences.bindings);
        self.suppressed_keys = [false; 10];
        self.own_ui_keys(ctx);
    }

    /// Capture owns input while settings suspend execution. A completed/cancelled
    /// event stays ineligible for gameplay until released, including on UI close.
    fn poll_binding_capture(&mut self, ctx: &egui::Context) {
        let Some(capture) = &mut self.capture else {
            return;
        };
        let slot = capture.slot;
        let result = ctx.input(|input| {
            capture.update(
                &input.events,
                input.keys_down.is_empty() && !input.pointer.any_down(),
            )
        });
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
                let changed = self.settings.preferences.bindings[slot] != key;
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
                        self.bindings_changed(ctx);
                    }
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
            self.sync_execution(true, true);
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

    /// Synchronizes session flags through its existing reanchoring APIs. The
    /// operation gate is constant-size and never performs guest or storage work.
    fn sync_execution(&mut self, focused: bool, visible: bool) -> bool {
        let paused = self.user_paused
            || self.restoring_save
            || self.execution_faulted
            || self.close_pending();
        if self.session.paused() != paused {
            self.session.toggle_pause();
            self.audio.clear();
        }
        let active =
            self.loaded && focused && visible && !self.operation_pending() && !self.settings_open;
        self.session.set_active(active);
        if !active || paused {
            self.session.release_all_buttons();
        }
        self.audio.set_playing(active && !paused);
        active && !paused
    }

    fn toggle_user_pause(&mut self) {
        if self.can_pause() {
            self.user_paused = !self.user_paused;
            self.replay_deadline = None;
            self.sync_execution(true, true);
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
        self.sync_execution(true, true);
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
            self.report_error(format!("BIOS picker failed: {error}"));
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
            self.report_error(format!(
                "BIOS must be 16 KiB (16384 bytes); received {} bytes.",
                bytes.len()
            ));
            return;
        }
        if self.loaded
            && (self.storage.busy || self.restoring_save || self.session.persistence_dirty())
        {
            self.pending_bios = Some(PendingBios {
                name: name.into(),
                bytes: bytes.to_vec(),
            });
            self.sync_execution(true, true);
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
                self.sync_execution(true, true);
                self.status = if self.loaded {
                    "BIOS replaced; game restarted"
                } else {
                    "BIOS ready; load a ROM to start"
                }
                .into();
            }
            Err(error) => self.report_error(format!("BIOS load failed: {error}")),
        }
    }

    /// Cancellation removes only the queued replacement, never an issued save write.
    fn cancel_replacement(&mut self) {
        if let Some(pending) = self.pending_rom.take() {
            self.restore_backup_override(pending.backup_override);
        }
        self.pending_bios = None;
        self.sync_execution(true, true);
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
        self.sync_execution(true, true);
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
            self.report_error(format!("ROM picker failed: {error}"));
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
                self.report_error(format!("ROM load failed: {error}"));
                return;
            }
            self.pending_rom = Some(PendingRom {
                name: name.to_owned(),
                bytes: bytes.to_vec(),
                backup_override,
                requires_bios: true,
            });
            self.status = "ROM ready; Load BIOS to start".into();
            self.sync_execution(true, true);
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
            self.sync_execution(true, true);
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
                self.report_error(format!("ROM load failed: {error}"));
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
        if let Some(completion) = self.storage.poll()
            && self.save_identity.as_ref() == Some(&completion.identity)
            && self.save_generation == completion.generation
        {
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
                            self.storage.status = if bytes.is_some() {
                                "Backup restored"
                            } else {
                                "No stored backup; new cartridge"
                            }
                            .into();
                            self.sync_execution(true, true);
                        }
                        Err(error) => {
                            self.storage.failed = true;
                            self.storage.status = error.into();
                        }
                    }
                }
                Outcome::Loaded(Err(error)) => {
                    self.storage.failed = true;
                    self.storage.status =
                        format!("Initial backup load failed: {error}; guest remains paused");
                }
                Outcome::Written(Ok(())) => {
                    self.session.acknowledge_save(completion.revision);
                    if let Some(revision) = completion.rtc_revision {
                        self.session.acknowledge_rtc(revision);
                    }
                    self.storage.failed = false;
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
                }
                Outcome::Imported(Ok(Some(bytes))) => {
                    self.save_import_open = false;
                    match self.session.import_save(&bytes) {
                        Ok(()) => {
                            self.storage.failed = false;
                            self.restoring_save = false;
                            // Restart guest registers so the imported score is read
                            // from cartridge bytes rather than overwritten by the old
                            // score still held in CPU registers. Keep execution paused.
                            self.reset_demo();
                            self.user_paused = true;
                            self.sync_execution(true, true);
                            self.storage.status =
                                "Imported; persistence pending — Resume to display".into();
                        }
                        Err(error) => {
                            self.storage.status = format!("Import rejected: {error}");
                            self.report_error(self.storage.status.clone());
                            self.report_error(self.storage.status.clone());
                        }
                    }
                }
                Outcome::Imported(Ok(None)) => {
                    self.save_import_open = false;
                    self.storage.status = "Import cancelled".into();
                }
                Outcome::Imported(Err(error)) => {
                    self.save_import_open = false;
                    self.storage.status = format!("Import rejected: {error}")
                }
                Outcome::Exported(Ok(true)) => {
                    self.storage.status =
                        "Export prepared; storage acknowledgement unchanged".into()
                }
                Outcome::Exported(Ok(false)) => self.storage.status = "Export cancelled".into(),
                Outcome::Exported(Err(error)) => {
                    self.storage.status = format!("Export failed: {error}");
                    self.report_error(self.storage.status.clone());
                }
            }
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
        if self.storage.busy {
            ctx.request_repaint_after(Duration::from_millis(20));
        }
    }

    /// Imports run with the guest paused and after any outstanding write. Exports
    /// include identity and the latest bytes, including data from a failed write.
    fn draw_save_controls(&mut self, ui: &mut egui::Ui) {
        if self.save_identity.is_none() {
            return;
        }
        ui.label(&self.storage.status);
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(!self.storage.busy, egui::Button::new("Retry save storage"))
                .clicked()
            {
                self.storage.failed = false;
                ui.ctx().request_repaint();
            }
            if ui
                .add_enabled(
                    self.can_import_save(),
                    egui::Button::new("Import cartridge save"),
                )
                .clicked()
            {
                self.import_cartridge_save(ui.ctx());
            }
            if ui
                .add_enabled(
                    self.can_export_save(),
                    egui::Button::new("Export cartridge save"),
                )
                .clicked()
            {
                self.export_cartridge_save(ui.ctx());
            }
        });
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

    /// Samples held host keys into the logical button state owned by the session.
    fn poll_keyboard(&mut self, ctx: &egui::Context) {
        ctx.input(|input| {
            for (slot, (button, key)) in BINDING_BUTTONS
                .into_iter()
                .zip(self.settings.preferences.bindings)
                .enumerate()
            {
                let down = input.key_down(key);
                if !down {
                    self.suppressed_keys[slot] = false;
                }
                self.session
                    .set_button(button, down && !self.suppressed_keys[slot]);
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
        self.audio.clear();
        self.session.reset();
        self.execution_faulted = false;
        self.notice = None;
        self.sync_execution(true, true);
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
                        self.sync_execution(true, true);
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
                        self.sync_execution(true, true);
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
                        self.sync_execution(true, true);
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
                        self.sync_execution(true, true);
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

        ui.horizontal(|ui| {
            if ui.button("Enable audio").clicked() {
                self.audio.start();
            }
            ui.checkbox(&mut self.audio.muted, "Mute");
            ui.add(egui::Slider::new(&mut self.audio.volume, 0.0..=1.0).text("Volume"));
        });
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
            ui.add(egui::Image::from_texture(texture).fit_to_exact_size(size));
        }
    }
}

impl eframe::App for GbaApp {
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
                Err(error) => self.report_error(format!("ROM read failed: {error}")),
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
                Some((_, Err(error))) => self.report_error(format!("BIOS read failed: {error}")),
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
                        self.report_error(format!("ROM read failed: {error}"));
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
        let focused = ctx.input(|input| input.focused);
        // Browser visibility is distinct from keyboard focus. A hidden tab must
        // suspend emulation even when the host throttles animation callbacks.
        #[cfg(target_arch = "wasm32")]
        let visible = web_sys::window()
            .and_then(|window| window.document())
            .is_some_and(|document| !document.hidden());
        #[cfg(not(target_arch = "wasm32"))]
        let visible = !ctx.input(|input| input.viewport().minimized.unwrap_or(false));
        if !(focused && visible) {
            self.capture = None;
        }
        self.poll_binding_capture(ctx);
        let running = self.sync_execution(focused, visible);
        if self.replay_deadline.is_some() && !(focused && visible) {
            self.replay_deadline = None;
            self.status =
                "Replay cancelled on focus loss; run it again to compare positions".into();
        }
        if running
            && self.replay_deadline.is_none()
            && self.capture.is_none()
            && !self.ui_keys_owned
            && !ctx.egui_wants_keyboard_input()
        {
            self.poll_keyboard(ctx);
        } else if self.ui_keys_owned || ctx.egui_wants_keyboard_input() {
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
                self.report_error(format!("Guest execution failed: {error}"));
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
        if self.loaded
            && !self.session.paused()
            && focused
            && visible
            && !self.operation_pending()
            && !self.settings_open
        {
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

/// Resizing must fill the available area without a scale cap or aspect distortion.
#[cfg(test)]
mod tests {
    use super::*;

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

#![forbid(unsafe_code)]

mod resampler;
pub use resampler::Resampler;

use gba_core::{CoreError, Machine, RunError, RunReport};
use std::time::Duration;

pub use gba_core::{CYCLES_PER_FRAME, Cycle, GBA_CLOCK_HZ, PCM_RATE, SCREEN_HEIGHT, SCREEN_WIDTH};

pub use gba_core::{BackupSelection, BackupType, Button, ButtonState};
pub use gba_core::{
    EEPROM8K_BYTES, EEPROM512_BYTES, FLASH64_BYTES, FLASH128_BYTES, SRAM_BYTES, SaveImage,
    SaveStatus,
};

pub const BUTTONS: [Button; 10] = [
    Button::A,
    Button::B,
    Button::Select,
    Button::Start,
    Button::Right,
    Button::Left,
    Button::Up,
    Button::Down,
    Button::R,
    Button::L,
];

/// Owns one core machine and session-level input and pause state.
///
/// This layer translates no host key codes; frontend adapters update logical buttons
/// through `set_button`, leaving the core independent of egui and other UI frameworks.
pub struct Session {
    machine: Machine,
    paused: bool,
    loaded: bool,
    frame_target: Cycle,
    active: bool,
    host_anchor: Option<Duration>,
    fractional_cycles: u128,
    slowed: bool,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            machine: Machine::new(),
            paused: false,
            loaded: false,
            frame_target: Cycle(0),
            active: true,
            host_anchor: None,
            fractional_cycles: 0,
            slowed: false,
        }
    }
}

impl Session {
    /// Captures cartridge bytes without advancing guest execution.
    pub fn save_image(&self) -> Option<SaveImage> {
        self.machine.save_image()
    }

    /// Reports cartridge save metadata without copying the owned backup bytes.
    pub fn save_status(&self) -> Option<SaveStatus> {
        self.machine.save_status()
    }

    /// Installs initial storage bytes before the first guest instruction.
    pub fn load_save(&mut self, bytes: &[u8]) -> Result<(), &'static str> {
        self.machine.load_save(bytes)
    }

    /// Applies an ordered import as a newer dirty cartridge revision.
    pub fn import_save(&mut self, bytes: &[u8]) -> Result<(), &'static str> {
        self.machine.import_save(bytes)
    }

    /// Completes a host write for exactly the snapshot revision it stored.
    pub fn acknowledge_save(&mut self, revision: u64) {
        self.machine.acknowledge_save(revision);
    }

    /// Drains core PCM for the frontend's streaming resampler without running CPU work.
    pub fn drain_pcm(&mut self, output: &mut Vec<f32>) {
        self.machine.drain_pcm(output);
    }

    /// Drains left/right frames for platform playback without advancing time.
    pub fn drain_stereo_pcm(&mut self, output: &mut Vec<[f32; 2]>) {
        self.machine.drain_stereo_pcm(output);
    }

    /// Reports core sample production, staging overflow and empty-FIFO consumption.
    pub fn pcm_counters(&self) -> (u64, u64, u64) {
        self.machine.pcm_counters()
    }

    /// Creates a fresh machine with all buttons released and execution unpaused.
    pub fn new() -> Self {
        Self::default()
    }

    /// Loads guest bytes into the owned machine and resets its execution state.
    pub fn load_rom(&mut self, rom: &[u8]) -> Result<(), CoreError> {
        self.load_rom_with_backup(rom, None)
    }

    /// Shares the core selection path with frontends using an explicit hardware override.
    pub fn load_rom_with_backup(
        &mut self,
        rom: &[u8],
        manual_override: Option<BackupType>,
    ) -> Result<(), CoreError> {
        self.machine.load_rom_with_backup(rom, manual_override)?;
        self.machine.release_all_buttons();
        self.paused = false;
        self.loaded = true;
        self.frame_target = Cycle(0);
        self.reanchor();
        Ok(())
    }

    /// Exposes cartridge identification independently of emulation and persistence.
    pub fn backup_selection(&self) -> &BackupSelection {
        self.machine.backup_selection()
    }

    /// Enables controlled diagnostic services after loading a test ROM. Reset
    /// retains this mapping; a subsequent cartridge load clears it.
    pub fn enable_test_firmware(&mut self) {
        self.machine.enable_test_firmware();
    }

    /// Advances one emulated frame toward an absolute deadline with bounded work.
    /// Used by deterministic headless checks; live frontends use `advance_host_time`.
    pub fn advance_frame(&mut self) -> Result<Option<RunReport>, RunError> {
        self.advance_frame_with_budget(200_000)
    }

    /// Advances a deterministic frame using the fixture's declared instruction cap.
    /// This keeps runaway guest work bounded independently of normal app pacing.
    pub fn advance_frame_with_budget(
        &mut self,
        instruction_limit: usize,
    ) -> Result<Option<RunReport>, RunError> {
        if self.paused || !self.active || !self.loaded {
            return Ok(None);
        }
        self.frame_target.0 += CYCLES_PER_FRAME;
        self.reanchor();
        self.machine
            .advance_to(self.frame_target, instruction_limit)
            .map(Some)
    }

    /// Converts an absolute monotonic host timestamp to an integer cycle deadline.
    /// Fractional cycles survive callbacks, so refresh rate does not change speed.
    /// At most two frames run per callback; excess host delay is dropped and reported
    /// as slow emulation rather than building an unbounded catch-up backlog.
    pub fn advance_host_time(&mut self, now: Duration) -> Result<Option<RunReport>, RunError> {
        self.advance_host_time_to(now, None)
    }

    /// Uses normal host pacing but stops at a replay's absolute cycle deadline.
    /// The final instruction may overshoot; later callbacks perform no more work.
    pub fn advance_host_time_until(
        &mut self,
        now: Duration,
        deadline: Cycle,
    ) -> Result<Option<RunReport>, RunError> {
        self.advance_host_time_to(now, Some(deadline))
    }

    fn advance_host_time_to(
        &mut self,
        now: Duration,
        deadline: Option<Cycle>,
    ) -> Result<Option<RunReport>, RunError> {
        if self.paused || !self.active || !self.loaded {
            self.reanchor();
            return Ok(None);
        }
        let Some(previous) = self.host_anchor.replace(now) else {
            return Ok(None);
        };
        let Some(elapsed) = now.checked_sub(previous) else {
            self.fractional_cycles = 0;
            return Ok(None);
        };
        let numerator = elapsed.as_nanos() * u128::from(GBA_CLOCK_HZ) + self.fractional_cycles;
        let due = numerator / 1_000_000_000;
        self.fractional_cycles = numerator % 1_000_000_000;
        let budget = u128::from(2 * CYCLES_PER_FRAME);
        self.slowed = due > budget;
        if self.slowed {
            self.fractional_cycles = 0;
        }
        self.frame_target.0 = self.frame_target.0.saturating_add(due.min(budget) as u64);
        if let Some(deadline) = deadline {
            self.frame_target = self.frame_target.min(deadline);
        }
        if self.machine.cycles() >= self.frame_target {
            return Ok(None);
        }
        self.machine
            .advance_to(self.frame_target, 200_000)
            .map(Some)
    }

    /// Clears the host time anchor at lifecycle boundaries, retaining guest time.
    fn reanchor(&mut self) {
        self.host_anchor = None;
        self.fractional_cycles = 0;
        self.slowed = false;
    }

    /// Suspends a hidden or unfocused frontend without changing its manual pause.
    /// Re-entry starts a new host-time anchor and never restores held buttons.
    pub fn set_active(&mut self, active: bool) {
        if self.active != active {
            self.active = active;
            self.machine.clear_pcm();
            self.release_all_buttons();
            self.reanchor();
        }
    }

    /// Reports whether the last active callback exceeded the bounded cycle budget.
    pub fn slowed(&self) -> bool {
        self.slowed
    }

    /// Schedules deterministic input in chronological guest-cycle order.
    pub fn set_button_at(
        &mut self,
        cycle: Cycle,
        button: Button,
        pressed: bool,
    ) -> Result<(), CoreError> {
        self.machine.set_button_at(cycle, button, pressed)
    }

    /// Inspects a guest mailbox without charging time or triggering device effects.
    pub fn inspect16(&self, address: u32) -> Result<u16, CoreError> {
        self.machine.inspect16(address)
    }

    /// Identifies the completed framebuffer independently of host redraw requests.
    pub fn framebuffer_generation(&self) -> u64 {
        self.machine.framebuffer_generation()
    }

    /// Returns the current framebuffer without copying the core-owned pixels.
    pub fn framebuffer(&self) -> &[u16] {
        self.machine.framebuffer()
    }

    /// Returns the number of guest instructions completed by the machine.
    pub fn executed_instructions(&self) -> usize {
        self.machine.executed_instructions()
    }

    /// Returns the current emulated hardware cycle position.
    pub fn cycles(&self) -> Cycle {
        self.machine.cycles()
    }

    /// Updates a host-independent logical button state.
    pub fn set_button(&mut self, button: Button, pressed: bool) {
        if self.active && !self.paused {
            self.machine.set_button(button, pressed);
        }
    }

    /// Returns whether a logical button is held.
    pub fn button_pressed(&self, button: Button) -> bool {
        self.machine.button_pressed(button)
    }

    /// Releases all logical buttons, for example after window focus is lost.
    pub fn release_all_buttons(&mut self) {
        self.machine.release_all_buttons();
    }

    /// Returns whether session execution is paused.
    pub fn paused(&self) -> bool {
        self.paused
    }

    /// Switches between paused and running session states.
    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
        self.machine.clear_pcm();
        self.release_all_buttons();
        self.reanchor();
    }

    /// Resets the core and clears input and pause state together.
    pub fn reset(&mut self) {
        self.machine.reset();
        self.machine.release_all_buttons();
        self.paused = false;
        self.frame_target = Cycle(0);
        self.reanchor();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    // Catches pre-pause PCM being replayed after resume or focus return. Existing
    // input/pacing tests preserve cycles but cannot observe stale sound staging.
    #[test]
    fn lifecycle_boundaries_discard_staged_audio() {
        let mut session = Session::new();
        session
            .load_rom(include_bytes!("../../../roms/pixels.gba"))
            .unwrap();
        session.advance_frame().unwrap();
        let mut samples = Vec::new();
        session.drain_pcm(&mut samples);
        assert!(!samples.is_empty());
        session.advance_frame().unwrap();
        session.toggle_pause();
        samples.clear();
        session.drain_pcm(&mut samples);
        assert!(samples.is_empty());
        session.toggle_pause();
        session.advance_frame().unwrap();
        session.set_active(false);
        session.drain_pcm(&mut samples);
        assert!(samples.is_empty());
        session.set_active(true);
        session.advance_frame().unwrap();
        session.reset();
        session.drain_pcm(&mut samples);
        assert!(samples.is_empty());
        assert_eq!(session.pcm_counters(), (0, 0, 0));
    }

    // Catches a replay running past its declared final checkpoint when a host
    // callback straddles that deadline. Refresh-rate equality alone misses this.
    #[test]
    fn host_pacing_stops_at_the_replay_deadline() {
        let mut session = Session::new();
        session
            .load_rom(include_bytes!("../../../roms/buttons.gba"))
            .unwrap();
        let deadline = Cycle(9 * CYCLES_PER_FRAME);
        for tick in 0..=10 {
            session
                .advance_host_time_until(Duration::from_nanos(tick * 1_000_000_000 / 60), deadline)
                .unwrap();
        }
        assert!(session.cycles() >= deadline && session.cycles().0 < deadline.0 + 40);
        assert_eq!(session.framebuffer_generation(), 9);
        assert_eq!(session.inspect16(0x03000006).unwrap(), 9);
    }

    // Catches one-frame-per-redraw pacing and fractional-time loss at 144 Hz.
    // Guest movement tests with explicit frame advances do not exercise host time.
    #[test]
    fn host_refresh_rates_produce_the_same_guest_position_and_timeline() {
        let run = |hz: u64| {
            let mut session = Session::new();
            session
                .load_rom(include_bytes!("../../../roms/buttons.gba"))
                .unwrap();
            session.set_button(Button::Right, true);
            for tick in 0..=hz {
                session
                    .advance_host_time(Duration::from_nanos(tick * 1_000_000_000 / hz))
                    .unwrap();
            }
            assert_eq!(session.machine.inspect16(0x03000002).unwrap(), 171);
            assert_eq!(session.machine.inspect16(0x03000006).unwrap(), 59);
            assert!(session.cycles().0 >= GBA_CLOCK_HZ && session.cycles().0 < GBA_CLOCK_HZ + 40);
            (session.cycles(), session.framebuffer().to_vec())
        };
        assert_eq!(run(60), run(144));
    }

    // Catches paused/hidden host time becoming catch-up work, or queued key presses
    // resurrecting after focus loss. The previous pause test has no host timeline.
    #[test]
    fn pause_and_focus_loss_release_input_and_reanchor_host_time() {
        let mut session = Session::new();
        session
            .load_rom(include_bytes!("../../../roms/buttons.gba"))
            .unwrap();
        session.advance_host_time(Duration::ZERO).unwrap();
        session
            .advance_host_time(Duration::from_millis(10))
            .unwrap();
        let before = session.cycles();
        session.set_button(Button::Right, true);
        session
            .set_button_at(Cycle(before.0 + CYCLES_PER_FRAME), Button::Right, true)
            .unwrap();
        session.toggle_pause();
        assert!(!session.button_pressed(Button::Right));
        session.advance_host_time(Duration::from_secs(100)).unwrap();
        session.toggle_pause();
        session.advance_host_time(Duration::from_secs(100)).unwrap();
        assert_eq!(session.cycles(), before);
        session.set_button(Button::Left, true);
        session.set_active(false);
        session.advance_host_time(Duration::from_secs(200)).unwrap();
        assert!(!session.button_pressed(Button::Left));
        session.set_active(true);
        session.advance_host_time(Duration::from_secs(200)).unwrap();
        assert_eq!(session.cycles(), before);
        session
            .advance_host_time(Duration::from_millis(200_050))
            .unwrap();
        assert!(!session.button_pressed(Button::Right));
        assert!(session.slowed());
        assert!(session.cycles().0 <= before.0 + 2 * CYCLES_PER_FRAME + 40);
        let reached = session.cycles();
        session
            .advance_host_time(Duration::from_millis(200_050))
            .unwrap();
        assert_eq!(session.cycles(), reached);
    }

    // Catches logical input stopping at the session while the actual guest sees
    // all keys released. The existing button-bitset tests cannot detect this.
    #[test]
    fn held_button_reaches_guest_and_moves_once_per_emulated_frame() {
        let mut session = Session::new();
        session
            .load_rom(include_bytes!("../../../roms/buttons.gba"))
            .unwrap();
        session.set_button(Button::Right, true);
        for frame in 1..=3 {
            session.advance_frame().unwrap();
            assert_eq!(session.machine.inspect16(0x03000002).unwrap(), 112 + frame);
            assert_eq!(session.machine.inspect16(0x03000006).unwrap(), frame);
        }
        session.release_all_buttons();
        session.advance_frame().unwrap();
        assert_eq!(session.machine.inspect16(0x03000002).unwrap(), 115);
    }

    #[test]
    fn paused_session_preserves_guest_time_and_resume_produces_a_frame() {
        let mut session = Session::new();
        let rom: Vec<u8> = [0xEAFFFFFEu32, 0, 0]
            .into_iter()
            .flat_map(u32::to_le_bytes)
            .collect();
        session.load_rom(&rom).unwrap();
        session.toggle_pause();
        assert!(session.advance_frame().unwrap().is_none());
        assert_eq!(session.cycles(), Cycle(0));
        session.toggle_pause();
        assert!(session.advance_frame().unwrap().is_some());
        assert!(session.cycles().0 >= CYCLES_PER_FRAME);
        assert_eq!(session.framebuffer_generation(), 1);
        session.reset();
        assert_eq!(session.cycles(), Cycle(0));
        assert_eq!(session.framebuffer_generation(), 0);
    }

    #[test]
    fn logical_buttons_can_be_pressed_and_released() {
        let mut state = ButtonState::default();

        state.set(Button::A, true);
        assert!(state.pressed(Button::A));

        state.set(Button::A, false);
        assert!(!state.pressed(Button::A));
    }

    #[test]
    fn release_all_clears_every_button() {
        let mut state = ButtonState::default();

        state.set(Button::A, true);
        state.set(Button::Right, true);
        state.release_all();

        assert!(BUTTONS.iter().all(|button| !state.pressed(*button)));
    }
}

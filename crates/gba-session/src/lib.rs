#![forbid(unsafe_code)]

use gba_core::{CoreError, Machine, RunError, RunReport};

pub use gba_core::{CYCLES_PER_FRAME, Cycle, GBA_CLOCK_HZ, SCREEN_HEIGHT, SCREEN_WIDTH};

/// The same assembled guest artifact is used by native, browser, and headless runs.
pub const PIXELS_ROM: &[u8] = include_bytes!("../../../roms/pixels.gba");

/// Host-independent identity for one of the ten physical GBA buttons.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    A = 0,
    B = 1,
    Select = 2,
    Start = 3,
    Right = 4,
    Left = 5,
    Up = 6,
    Down = 7,
    R = 8,
    L = 9,
}

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

/// Compact bitset for the current logical button state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ButtonState(u16);

impl ButtonState {
    /// Sets or clears the bit associated with `button`.
    pub fn set(&mut self, button: Button, pressed: bool) {
        let mask = 1_u16 << button as u8;

        if pressed {
            self.0 |= mask;
        } else {
            self.0 &= !mask;
        }
    }

    /// Returns whether `button` is currently held.
    pub fn pressed(self, button: Button) -> bool {
        self.0 & (1_u16 << button as u8) != 0
    }

    /// Releases every button, including keys that may have remained held on focus loss.
    pub fn release_all(&mut self) {
        self.0 = 0;
    }
}

/// Owns one core machine and session-level input and pause state.
///
/// This layer translates no host key codes; frontend adapters update logical buttons
/// through `set_button`, leaving the core independent of egui and other UI frameworks.
pub struct Session {
    machine: Machine,
    buttons: ButtonState,
    paused: bool,
    loaded: bool,
    frame_target: Cycle,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            machine: Machine::new(),
            buttons: ButtonState::default(),
            paused: false,
            loaded: false,
            frame_target: Cycle(0),
        }
    }
}

impl Session {
    /// Creates a fresh machine with all buttons released and execution unpaused.
    pub fn new() -> Self {
        Self::default()
    }

    /// Loads guest bytes into the owned machine and resets its execution state.
    pub fn load_rom(&mut self, rom: &[u8]) -> Result<(), CoreError> {
        self.machine.load_rom(rom)?;
        self.buttons.release_all();
        self.paused = false;
        self.loaded = true;
        self.frame_target = Cycle(0);
        Ok(())
    }

    /// Advances one emulated frame toward an absolute deadline with bounded work.
    /// Host wake scheduling belongs to the app; master-clock input pacing follows in Slice 2.
    pub fn advance_frame(&mut self) -> Result<Option<RunReport>, RunError> {
        if self.paused || !self.loaded {
            return Ok(None);
        }
        self.frame_target.0 += CYCLES_PER_FRAME;
        self.machine
            .advance_to(self.frame_target, 200_000)
            .map(Some)
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
        self.buttons.set(button, pressed);
    }

    /// Returns whether a logical button is held.
    pub fn button_pressed(&self, button: Button) -> bool {
        self.buttons.pressed(button)
    }

    /// Releases all logical buttons, for example after window focus is lost.
    pub fn release_all_buttons(&mut self) {
        self.buttons.release_all();
    }

    /// Returns whether session execution is paused.
    pub fn paused(&self) -> bool {
        self.paused
    }

    /// Switches between paused and running session states.
    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    /// Resets the core and clears input and pause state together.
    pub fn reset(&mut self) {
        self.machine.reset();
        self.buttons.release_all();
        self.paused = false;
        self.frame_target = Cycle(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

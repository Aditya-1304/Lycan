#![forbid(unsafe_code)]

use gba_core::Machine;

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
}

impl Default for Session {
    fn default() -> Self {
        Self {
            machine: Machine::new(),
            buttons: ButtonState::default(),
            paused: false,
        }
    }
}

impl Session {
    /// Creates a fresh machine with all buttons released and execution unpaused.
    pub fn new() -> Self {
        Self::default()
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

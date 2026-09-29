#![forbid(unsafe_code)]

use gba_core::Machine;

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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ButtonState(u16);

impl ButtonState {
    pub fn set(&mut self, button: Button, pressed: bool) {
        let mask = 1_u16 << button as u8;

        if pressed {
            self.0 |= mask;
        } else {
            self.0 &= !mask;
        }
    }

    pub fn pressed(self, button: Button) -> bool {
        self.0 & (1_u16 << button as u8) != 0
    }

    pub fn release_all(&mut self) {
        self.0 = 0;
    }
}

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
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_button(&mut self, button: Button, pressed: bool) {
        self.buttons.set(button, pressed);
    }

    pub fn button_pressed(&self, button: Button) -> bool {
        self.buttons.pressed(button)
    }

    pub fn release_all_buttons(&mut self) {
        self.buttons.release_all();
    }

    pub fn paused(&self) -> bool {
        self.paused
    }

    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

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

//! Logical keypad input shared by hosts and the emulated hardware.

/// Logical GBA buttons in their KEYINPUT bit order, independent of host keys.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    A,
    B,
    Select,
    Start,
    Right,
    Left,
    Up,
    Down,
    R,
    L,
}

/// Pressed-button bitset; conversion to active-low hardware bits stays in the core.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ButtonState(pub(crate) u16);
impl ButtonState {
    /// Updates one logical button without affecting the other nine buttons.
    pub fn set(&mut self, button: Button, pressed: bool) -> bool {
        let mask = 1 << button as u8;
        let before = self.0;

        if pressed {
            self.0 |= mask;
        } else {
            self.0 &= !mask;
        }

        self.0 != before
    }
    /// Reports whether the logical button is pressed.
    pub fn pressed(self, button: Button) -> bool {
        self.0 & (1 << button as u8) != 0
    }
    /// Releases all logical buttons.
    pub fn release_all(&mut self) {
        self.0 = 0;
    }
}

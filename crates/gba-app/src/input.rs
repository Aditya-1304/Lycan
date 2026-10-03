//! Host-key mapping and capture; no machine execution or persistence lives here.

use crate::settings::supported_key;
use eframe::egui::{self, Key};
use gba_session::Button;

/// Exclusive owner of one host input batch. UI interaction does not itself pause
/// the machine; settings and file operations have separate execution blockers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InputOwner {
    Gameplay,
    #[default]
    Ui,
    Rebinding,
}

#[cfg(target_arch = "wasm32")]
pub mod browser;

/// Persisted slots are application order, independent of KEYINPUT bit order.
pub const BINDING_BUTTONS: [Button; 10] = [
    Button::A,
    Button::B,
    Button::L,
    Button::R,
    Button::Start,
    Button::Select,
    Button::Up,
    Button::Down,
    Button::Left,
    Button::Right,
];

/// A complete edit preserves uniqueness by exchanging an occupied key with the
/// destination's old key. Validation precedes mutation; reserved keys never enter
/// either gameplay or the preference record. The result identifies a displaced slot.
pub fn remap(keys: &mut [Key; 10], slot: usize, key: Key) -> Result<Option<usize>, ()> {
    if slot >= keys.len() || !supported_key(key) {
        return Err(());
    }
    let displaced = keys
        .iter()
        .position(|current| *current == key)
        .filter(|other| *other != slot);
    if let Some(other) = displaced {
        keys.swap(slot, other);
    } else {
        keys[slot] = key;
    }
    Ok(displaced)
}

/// Shared labels are rebuilt only after loading or editing bindings. Tooltips use
/// the full logical-key name; the deck/strip use the same compact formatter.
pub struct BindingLabel {
    pub full: &'static str,
    pub legend: String,
}

pub fn binding_labels(keys: &[Key; 10]) -> [BindingLabel; 10] {
    std::array::from_fn(|slot| {
        let key = keys[slot];
        let compact = match key {
            Key::Backspace => "Bksp",
            Key::PageUp => "PgUp",
            Key::PageDown => "PgDn",
            Key::ArrowUp => "Up",
            Key::ArrowDown => "Down",
            Key::ArrowLeft => "Left",
            Key::ArrowRight => "Right",
            _ => key.name(),
        };
        BindingLabel {
            full: key.name(),
            legend: format!("{} ({compact})", button_name(BINDING_BUTTONS[slot])),
        }
    })
}

pub fn button_name(button: Button) -> &'static str {
    match button {
        Button::A => "A",
        Button::B => "B",
        Button::L => "L",
        Button::R => "R",
        Button::Start => "Start",
        Button::Select => "Select",
        Button::Up => "Up",
        Button::Down => "Down",
        Button::Left => "Left",
        Button::Right => "Right",
    }
}

/// Capture is armed only after activation has been released. Events are borrowed
/// from egui; capture adds no event clone, input collection or gameplay-loop work.
pub struct Capture {
    pub slot: usize,
    armed: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CaptureResult {
    Waiting,
    Unsupported,
    Cancelled,
    Key(Key),
}

impl Capture {
    pub fn new(slot: usize, activation_released: bool) -> Self {
        Self {
            slot,
            armed: activation_released,
        }
    }

    pub fn update(&mut self, events: &[egui::Event], activation_released: bool) -> CaptureResult {
        for event in events {
            if matches!(
                event,
                egui::Event::Key {
                    key: Key::Escape,
                    pressed: true,
                    repeat: false,
                    ..
                }
            ) {
                return CaptureResult::Cancelled;
            }
        }
        if !self.armed {
            self.armed = activation_released;
            return CaptureResult::Waiting;
        }
        for event in events {
            if let egui::Event::Key {
                key,
                pressed: true,
                repeat: false,
                modifiers,
                ..
            } = event
            {
                return if modifiers.is_none() && supported_key(*key) {
                    CaptureResult::Key(*key)
                } else {
                    CaptureResult::Unsupported
                };
            }
        }
        CaptureResult::Waiting
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::DEFAULT_BINDINGS;

    // Rebinding A to B's key must preserve a complete unique mapping, including
    // the displaced assignment. Preference validation only repairs corrupt loads.
    #[test]
    fn conflicting_key_swaps_atomically_and_reserved_keys_do_not_mutate() {
        let mut keys = DEFAULT_BINDINGS;
        assert_eq!(remap(&mut keys, 0, Key::X), Ok(Some(1)));
        assert_eq!((keys[0], keys[1]), (Key::X, Key::Z));
        let swapped = keys;
        assert_eq!(remap(&mut keys, 0, Key::X), Ok(None));
        assert!(remap(&mut keys, 0, Key::F1).is_err());
        assert_eq!(keys, swapped);
        assert_eq!(remap(&mut keys, 9, Key::Num1), Ok(None));
        assert_eq!(keys[9], Key::Num1);
        for slot in 0..10 {
            assert!(!keys[..slot].contains(&keys[slot]));
        }
    }

    fn press(key: Key, repeat: bool, modifiers: egui::Modifiers) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat,
            modifiers,
        }
    }

    // Keyboard activation, repeats and modifier combinations must not complete
    // capture; reserved keys must explain rejection while Escape cancels.
    #[test]
    fn capture_waits_for_activation_release_and_only_accepts_fresh_supported_press() {
        let mut capture = Capture::new(0, false);
        let enter = press(Key::Enter, false, egui::Modifiers::NONE);
        assert_eq!(
            capture.update(std::slice::from_ref(&enter), false),
            CaptureResult::Waiting
        );
        assert_eq!(capture.update(&[], true), CaptureResult::Waiting);
        assert_eq!(
            capture.update(&[press(Key::Z, true, egui::Modifiers::NONE)], false),
            CaptureResult::Waiting
        );
        assert_eq!(
            capture.update(&[press(Key::Z, false, egui::Modifiers::CTRL)], false),
            CaptureResult::Unsupported
        );
        assert_eq!(
            capture.update(&[press(Key::F11, false, egui::Modifiers::NONE)], false),
            CaptureResult::Unsupported
        );
        assert_eq!(
            capture.update(&[enter], false),
            CaptureResult::Key(Key::Enter)
        );
        assert_eq!(
            capture.update(&[press(Key::Escape, false, egui::Modifiers::NONE)], false),
            CaptureResult::Cancelled
        );
    }
}

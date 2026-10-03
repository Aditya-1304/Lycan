//! Application preferences only. Cartridge data remains on the save adapter path.
use eframe::egui::Key;
use serde::{Deserialize, Serialize};

/// Stable key shared by native eframe storage and browser local storage.
const STORAGE_KEY: &str = "lycan.preferences";
const VERSION: u32 = 1;

/// Slots follow the application mapping, independent of hardware button ordering.
pub const DEFAULT_BINDINGS: [Key; 10] = [
    Key::Z,
    Key::X,
    Key::A,
    Key::S,
    Key::Enter,
    Key::Backspace,
    Key::ArrowUp,
    Key::ArrowDown,
    Key::ArrowLeft,
    Key::ArrowRight,
];

/// Display policy; fullscreen is intentionally transient and never serialized.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Scaling {
    #[default]
    Fit,
    Integer,
}

/// Version-one preference payload. Absent optional fields use product defaults.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub version: u32,
    #[serde(with = "stored_bindings")]
    pub bindings: [Key; 10],
    pub volume: f32,
    pub muted: bool,
    pub legend_visible: bool,
    pub scaling: Scaling,
}

/// Store logical names rather than enum discriminants. A damaged group decodes
/// to a reserved sentinel, allowing validation to repair it without losing audio
/// or display choices. Allocation occurs only on preference load/save.
mod stored_bindings {
    use super::*;

    pub fn serialize<S: serde::Serializer>(
        keys: &[Key; 10],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        keys.iter()
            .copied()
            .map(Key::name)
            .collect::<Vec<_>>()
            .serialize(serializer)
    }

    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<[Key; 10], D::Error> {
        let names = Vec::<String>::deserialize(deserializer)?;
        let mut keys = [Key::Escape; 10];
        if names.len() == keys.len() {
            for (key, name) in keys.iter_mut().zip(names) {
                *key = Key::from_name(&name).unwrap_or(Key::Escape);
            }
        }
        Ok(keys)
    }
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            version: VERSION,
            bindings: DEFAULT_BINDINGS,
            volume: 0.5,
            muted: false,
            legend_visible: true,
            scaling: Scaling::Fit,
        }
    }
}

/// Loaded preferences plus a simple guard against overwriting a future format.
#[derive(Default)]
pub struct Settings {
    pub preferences: Preferences,
    pub warning: Option<String>,
    writes_blocked: bool,
}

/// Parse the version independently so unknown future fields cannot defeat the guard.
#[derive(Deserialize)]
struct VersionHeader {
    version: u32,
}

impl Settings {
    /// Missing storage is usable in memory; malformed data never reaches gameplay.
    pub fn load(storage: Option<&dyn eframe::Storage>) -> Self {
        let Some(storage) = storage else {
            return Self {
                warning: Some(
                    "Preference storage unavailable; settings may not survive restart.".into(),
                ),
                ..Self::default()
            };
        };
        if storage.get_string(STORAGE_KEY).is_none() {
            return Self::default();
        }
        let Some(header) = eframe::get_value::<VersionHeader>(storage, STORAGE_KEY) else {
            return Self {
                warning: Some("Preferences could not be read; using defaults.".into()),
                ..Self::default()
            };
        };
        if header.version > VERSION {
            return Self { writes_blocked: true,
                warning: Some("Preferences were saved by a newer Lycan version; using defaults without overwriting them.".into()),
                ..Self::default() };
        }
        if header.version != VERSION {
            return Self {
                warning: Some("Unsupported preference version; using defaults.".into()),
                ..Self::default()
            };
        }
        let Some(mut preferences) = eframe::get_value::<Preferences>(storage, STORAGE_KEY) else {
            return Self {
                warning: Some("Preferences could not be read; using defaults.".into()),
                ..Self::default()
            };
        };
        let warning = preferences.validate().then(|| {
            "Invalid preferences repaired; unsupported or duplicate bindings reset, and volume validated."
                .into()
        });
        Self {
            preferences,
            warning,
            writes_blocked: false,
        }
    }

    /// Serialize only through eframe's lifecycle, never during drawing or dragging.
    pub fn save(&self, storage: &mut dyn eframe::Storage, volume: f32, muted: bool) {
        if self.writes_blocked {
            return;
        }
        let mut preferences = self.preferences.clone();
        preferences.volume = volume;
        preferences.muted = muted;
        preferences.validate();
        eframe::set_value(storage, STORAGE_KEY, &preferences);
    }

    pub fn writes_blocked(&self) -> bool {
        self.writes_blocked
    }

    /// Explicit recovery authorizes subsequent normal saves to replace a future record.
    pub fn reset_saved_settings(&mut self) {
        *self = Self::default();
    }
}

impl Preferences {
    /// Validate the whole binding group; corrupt data must not be silently remapped.
    fn validate(&mut self) -> bool {
        let invalid_bindings = self
            .bindings
            .iter()
            .enumerate()
            .any(|(slot, key)| !supported_key(*key) || self.bindings[..slot].contains(key));
        if invalid_bindings {
            self.bindings = DEFAULT_BINDINGS;
        }
        let invalid_volume = !self.volume.is_finite() || !(0.0..=1.0).contains(&self.volume);
        self.volume = if self.volume.is_finite() {
            self.volume.clamp(0.0, 1.0)
        } else {
            0.5
        };
        invalid_bindings || invalid_volume
    }
}

/// Single logical keys supported by the pinned egui integration. Application
/// shortcuts and modifiers stay outside the gameplay binding domain.
pub(crate) fn supported_key(key: Key) -> bool {
    use Key::*;
    matches!(
        key,
        A | B
            | C
            | D
            | E
            | F
            | G
            | H
            | I
            | J
            | K
            | L
            | M
            | N
            | O
            | P
            | Q
            | R
            | S
            | T
            | U
            | V
            | W
            | X
            | Y
            | Z
            | Num0
            | Num1
            | Num2
            | Num3
            | Num4
            | Num5
            | Num6
            | Num7
            | Num8
            | Num9
            | ArrowUp
            | ArrowDown
            | ArrowLeft
            | ArrowRight
            | Space
            | Enter
            | Backspace
            | Tab
            | Insert
            | Delete
            | Home
            | End
            | PageUp
            | PageDown
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct MemoryStorage(Option<String>);
    impl eframe::Storage for MemoryStorage {
        fn get_string(&self, key: &str) -> Option<String> {
            (key == STORAGE_KEY).then(|| self.0.clone()).flatten()
        }
        fn set_string(&mut self, key: &str, value: String) {
            assert_eq!(key, STORAGE_KEY);
            self.0 = Some(value);
        }
        fn remove_string(&mut self, _key: &str) {
            self.0 = None;
        }
        fn flush(&mut self) {}
    }

    // Catches silently dropping edits across the standard eframe save/load lifecycle.
    #[test]
    fn preferences_round_trip_and_missing_fields_use_defaults() {
        let mut storage = MemoryStorage::default();
        let mut settings = Settings::load(Some(&storage));
        settings.preferences.legend_visible = false;
        settings.preferences.scaling = Scaling::Integer;
        settings.save(&mut storage, 0.25, true);
        let loaded = Settings::load(Some(&storage));
        assert_eq!(loaded.preferences.volume, 0.25);
        assert!(loaded.preferences.muted);
        assert!(!loaded.preferences.legend_visible);
        assert_eq!(loaded.preferences.scaling, Scaling::Integer);
        storage.0 = Some("(version:1,muted:true)".into());
        let loaded = Settings::load(Some(&storage));
        assert!(loaded.preferences.muted);
        assert_eq!(loaded.preferences.bindings, DEFAULT_BINDINGS);
        assert_eq!(loaded.preferences.volume, 0.5);
    }

    // Catches a downgraded binary destroying newer settings, even with unknown fields.
    #[test]
    fn newer_format_is_preserved_while_using_safe_defaults() {
        let mut storage = MemoryStorage(Some("(version:2,future_field:42)".into()));
        let original = storage.0.clone();
        let loaded = Settings::load(Some(&storage));
        assert_eq!(loaded.preferences, Preferences::default());
        assert!(loaded.warning.is_some());
        loaded.save(&mut storage, 0.2, true);
        assert_eq!(storage.0, original);
    }

    // Catches corrupt host keys and gain reaching gameplay instead of group recovery.
    #[test]
    fn invalid_records_recover_without_blocking_preferences() {
        let mut storage = MemoryStorage::default();
        let mut record = Preferences {
            volume: 8.0,
            ..Preferences::default()
        };
        record.bindings[0] = Key::F1;
        eframe::set_value(&mut storage, STORAGE_KEY, &record);
        let loaded = Settings::load(Some(&storage));
        assert_eq!(loaded.preferences.volume, 1.0);
        assert_eq!(loaded.preferences.bindings, DEFAULT_BINDINGS);
        assert!(loaded.warning.is_some());
        record.bindings = [Key::Z; 10];
        record.volume = f32::NAN;
        eframe::set_value(&mut storage, STORAGE_KEY, &record);
        let loaded = Settings::load(Some(&storage));
        assert_eq!(loaded.preferences.bindings, DEFAULT_BINDINGS);
        assert_eq!(loaded.preferences.volume, 0.5);
        // A damaged binding group must not discard otherwise valid audio settings.
        storage.0 = Some("(version:1,bindings:[\"unknown key\"],muted:true,volume:0.25)".into());
        let loaded = Settings::load(Some(&storage));
        assert!(loaded.preferences.muted);
        assert_eq!(loaded.preferences.volume, 0.25);
        assert_eq!(loaded.preferences.bindings, DEFAULT_BINDINGS);
        assert!(loaded.warning.is_some());
        storage.0 = Some("broken record".into());
        let loaded = Settings::load(Some(&storage));
        assert!(loaded.warning.is_some());
        loaded.save(&mut storage, 0.5, false);
        assert_eq!(
            Settings::load(Some(&storage)).preferences,
            Preferences::default()
        );
        assert!(
            Settings::load(Some(&MemoryStorage::default()))
                .warning
                .is_none()
        );
        assert_eq!(Settings::load(None).preferences, Preferences::default());
    }
}

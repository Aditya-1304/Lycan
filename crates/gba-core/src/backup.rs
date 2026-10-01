//! Cartridge configuration only; save-bus protocols and persistence are separate.

/// Save hardware family. EEPROM capacity is resolved by serial commands or an explicit override.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackupType {
    /// Explicitly forced cartridge with no save hardware.
    None,
    Sram,
    Eeprom,
    /// Validated overrides for cartridges whose serial size detection is uncertain.
    Eeprom512,
    Eeprom8k,
    Flash64,
    Flash128,
}

/// ROM identification evidence, retained even when a manual override is used.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum BackupDetection {
    /// No valid supported library identifier was found.
    #[default]
    Unknown,
    /// All recognized identifiers agree on one hardware family.
    Identified(BackupType),
    /// Distinct hardware families in stable order; no precedence is inferred.
    Ambiguous(Vec<BackupType>),
}

/// Load-time selection shared by every frontend and future save implementation.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BackupSelection {
    /// Unmodified identification evidence from the loaded ROM.
    pub detection: BackupDetection,
    /// Explicit host choice, taking precedence without hiding ROM evidence.
    pub manual_override: Option<BackupType>,
}
impl BackupSelection {
    /// Unknown and conflicting images remain unresolved until explicitly overridden.
    pub fn selected(&self) -> Option<BackupType> {
        self.manual_override.or(match self.detection {
            BackupDetection::Identified(kind) => Some(kind),
            _ => None,
        })
    }
}

/// Validates text-based overrides without silently falling back to autodetection.
impl std::str::FromStr for BackupType {
    type Err = &'static str;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "none" => Ok(Self::None),
            "sram" => Ok(Self::Sram),
            "eeprom" => Ok(Self::Eeprom),
            "eeprom512" => Ok(Self::Eeprom512),
            "eeprom8k" => Ok(Self::Eeprom8k),
            "flash64" => Ok(Self::Flash64),
            "flash128" => Ok(Self::Flash128),
            _ => Err(
                "backup type must be none, sram, eeprom, eeprom512, eeprom8k, flash64 or flash128",
            ),
        }
    }
}

/// Checks every byte offset for a three-digit version or the homebrew `nnn` marker.
fn contains_signature(rom: &[u8], prefix: &[u8]) -> bool {
    let length = prefix.len() + 3;

    rom.windows(length).any(|window| {
        if !window.starts_with(prefix) {
            return false;
        }

        let version = &window[prefix.len()..];

        version == b"nnn" || version.iter().all(u8::is_ascii_digit)
    })
}

/// Inspects library signatures without consulting titles or save-file contents.
pub fn detect_backup(rom: &[u8]) -> BackupDetection {
    let signatures: &[(BackupType, &[&[u8]])] = &[
        (BackupType::Sram, &[b"SRAM_V", b"SRAM_F_V"]),
        (BackupType::Eeprom, &[b"EEPROM_V"]),
        (BackupType::Flash64, &[b"FLASH_V", b"FLASH512_V"]),
        (BackupType::Flash128, &[b"FLASH1M_V"]),
    ];
    let mut found = Vec::new();
    for &(kind, aliases) in signatures {
        if aliases.iter().any(|prefix| contains_signature(rom, prefix)) {
            found.push(kind);
        }
    }
    match found.as_slice() {
        [] => BackupDetection::Unknown,
        [kind] => BackupDetection::Identified(*kind),
        _ => BackupDetection::Ambiguous(found),
    }
}

#[cfg(test)]
mod tests {
    use super::{BackupDetection, BackupType, detect_backup};

    #[test]
    fn detects_versioned_markers_at_every_word_offset() {
        for offset in 0..4 {
            let mut rom = vec![0; offset];
            rom.extend_from_slice(b"FLASH1M_V123");

            assert_eq!(
                detect_backup(&rom),
                BackupDetection::Identified(BackupType::Flash128),
                "marker at byte offset {offset} should be detected"
            );
        }
    }

    #[test]
    fn detects_numeric_signature_versions() {
        assert_eq!(
            detect_backup(b"EEPROM_V123"),
            BackupDetection::Identified(BackupType::Eeprom)
        );
    }

    #[test]
    fn preserves_homebrew_placeholder_versions() {
        assert_eq!(
            detect_backup(b"SRAM_Vnnn"),
            BackupDetection::Identified(BackupType::Sram)
        );
    }

    #[test]
    fn rejects_malformed_signature_versions() {
        assert_eq!(detect_backup(b"EEPROM_V12x"), BackupDetection::Unknown);
    }

    #[test]
    fn rejects_truncated_signature_versions() {
        assert_eq!(detect_backup(b"FLASH512_V12"), BackupDetection::Unknown);
    }

    #[test]
    fn reports_distinct_save_families_as_ambiguous() {
        assert_eq!(
            detect_backup(b"SRAM_V001\0\0\0FLASH_V002"),
            BackupDetection::Ambiguous(vec![BackupType::Sram, BackupType::Flash64])
        );
    }
}

//! Cartridge configuration only; save-bus protocols and persistence are separate.

/// Save hardware family. EEPROM capacity is resolved by its later serial protocol.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackupType {
    /// Explicitly forced cartridge with no save hardware.
    None,
    Sram,
    Eeprom,
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
            "flash64" => Ok(Self::Flash64),
            "flash128" => Ok(Self::Flash128),
            _ => Err("backup type must be none, sram, eeprom, flash64 or flash128"),
        }
    }
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
        // SDK identifiers are word-aligned and carry a three-digit version.
        // The documented homebrew placeholder "nnn" is also accepted.
        let matches = (0..rom.len()).step_by(4).any(|offset| {
            aliases.iter().any(|prefix| {
                let tail = &rom[offset..];
                tail.starts_with(prefix)
                    && tail
                        .get(prefix.len()..prefix.len() + 3)
                        .is_some_and(|version| {
                            version == b"nnn" || version.iter().all(u8::is_ascii_digit)
                        })
            })
        });
        if matches {
            found.push(kind);
        }
    }
    match found.as_slice() {
        [] => BackupDetection::Unknown,
        [kind] => BackupDetection::Identified(*kind),
        _ => BackupDetection::Ambiguous(found),
    }
}

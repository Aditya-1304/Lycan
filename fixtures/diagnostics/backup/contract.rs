// Load-time contract: no guest execution or persistence is required for detection.
use gba_core::{BackupDetection, BackupType, Machine};

pub fn verify() {
    let cases: &[(&str, &[u8], Option<BackupType>)] = &[
        ("sram", include_bytes!("sram.gba"), Some(BackupType::Sram)),
        (
            "sram-fast",
            include_bytes!("sram-fast.gba"),
            Some(BackupType::Sram),
        ),
        (
            "eeprom",
            include_bytes!("eeprom.gba"),
            Some(BackupType::Eeprom),
        ),
        (
            "flash64",
            include_bytes!("flash64.gba"),
            Some(BackupType::Flash64),
        ),
        (
            "flash64-old",
            include_bytes!("flash64-old.gba"),
            Some(BackupType::Flash64),
        ),
        (
            "flash128",
            include_bytes!("flash128.gba"),
            Some(BackupType::Flash128),
        ),
        (
            "same-family",
            include_bytes!("same-family.gba"),
            Some(BackupType::Flash64),
        ),
        ("unknown", include_bytes!("unknown.gba"), None),
        ("malformed", include_bytes!("malformed.gba"), None),
    ];
    let mut machine = Machine::new();
    machine.load_rom(include_bytes!("override.gba")).unwrap();
    assert_eq!(
        machine.backup_selection().selected(),
        Some(BackupType::Eeprom)
    );
    machine
        .load_rom_with_backup(include_bytes!("unknown.gba"), Some(BackupType::None))
        .unwrap();
    assert_eq!(
        machine.backup_selection().selected(),
        Some(BackupType::None)
    );
    for &(name, bytes, expected) in cases {
        machine.load_rom(bytes).unwrap();
        assert_eq!(machine.backup_selection().selected(), expected, "{name}");
        if expected.is_none() {
            assert_eq!(
                machine.backup_selection().detection,
                BackupDetection::Unknown
            );
        }
        assert_eq!(machine.executed_instructions(), 0);
    }
    let conflict = include_bytes!("conflicting.gba");
    machine.load_rom(conflict).unwrap();
    assert_eq!(
        machine.backup_selection().detection,
        BackupDetection::Ambiguous(vec![BackupType::Sram, BackupType::Flash128])
    );
    assert_eq!(machine.backup_selection().selected(), None);
    for bytes in [
        conflict.as_slice(),
        include_bytes!("unknown.gba").as_slice(),
        include_bytes!("override.gba").as_slice(),
    ] {
        machine
            .load_rom_with_backup(bytes, Some(BackupType::Flash64))
            .unwrap();
        let selection = machine.backup_selection().clone();
        assert_eq!(selection.selected(), Some(BackupType::Flash64));
        machine.reset();
        assert_eq!(machine.backup_selection(), &selection);
        assert!(
            machine
                .load_rom_with_backup(&[], Some(BackupType::Sram))
                .is_err()
        );
        assert_eq!(machine.backup_selection(), &selection);
    }
    machine.load_rom(include_bytes!("unknown.gba")).unwrap();
    assert_eq!(machine.backup_selection().selected(), None);
    assert!("automatic-ish".parse::<BackupType>().is_err());
    assert_eq!("none".parse::<BackupType>().unwrap(), BackupType::None);
}

//! Original guest contract: serial read/write, first/last block isolation and
//! recovery are checked through Session, with exact completed scanout evidence.
use gba_session::{BackupType, Button, SaveImage, Session};

pub const SMALL_ROM: &[u8] = include_bytes!("../eeprom512-score.gba");
pub const LARGE_ROM: &[u8] = include_bytes!("../eeprom8k-score.gba");

/// Bound the running score scene to three frames and require every pixel,
/// mailbox and physical backup byte. Reaching an arbitrary loop is insufficient.
fn run(rom: &[u8], capacity: usize, saved: Option<&[u8]>, press: bool, score: u16) -> SaveImage {
    let mut session = Session::new();
    session.load_rom(rom).unwrap();
    assert_eq!(
        session.backup_selection().selected(),
        Some(BackupType::Eeprom)
    );
    if let Some(bytes) = saved {
        session.load_save(bytes).unwrap();
    }
    session.set_button(Button::A, press);
    let mut pc = 0;
    for _ in 0..3 {
        pc = session
            .advance_frame_with_budget(200_000)
            .unwrap()
            .unwrap()
            .instruction_address;
    }
    // Polling or drawing may straddle the host frame boundary. The source label
    // ranges exclude startup and failure; complete outputs below prove success.
    assert!(
        ((0x08000134..=0x0800018c).contains(&pc) || (0x0800019c..=0x0800022c).contains(&pc))
            && pc % 4 == 0,
        "PC {pc:#010x}"
    );
    assert!(session.cycles().0 <= 843000);
    assert_eq!(session.inspect16(0x03000000).unwrap(), 0x76);
    assert_eq!(session.inspect16(0x03000002).unwrap(), score);
    assert_eq!(session.inspect16(0x03000004).unwrap(), score + 16);
    for (index, &pixel) in session.framebuffer().iter().enumerate() {
        let expected = if index % 240 < usize::from(score) * 8 {
            match index / 240 {
                0..=7 => 0x03e0,
                8..=15 => 0x7c00,
                _ => 0,
            }
        } else {
            0
        };
        assert_eq!(pixel, expected, "EEPROM framebuffer pixel {index}");
    }
    let image = session.save_image().unwrap();
    assert_eq!(image.bytes.len(), capacity);
    assert_eq!(
        &image.bytes[..8],
        &[0x5a, score as u8 + 16, 0, 0, 0, 0, 0, 0]
    );
    assert_eq!(
        &image.bytes[capacity - 8..],
        &[0xa5, score as u8, 0, 0, 0, 0, 0, 0]
    );
    assert!(
        image.bytes[8..capacity - 8]
            .iter()
            .all(|&byte| byte == 0xff)
    );
    #[cfg(not(target_arch = "wasm32"))]
    println!(
        "EEPROM guest PASS: capacity={capacity} score={score} PC={pc:#010x} cycles={} instructions={} revision={} dirty={}",
        session.cycles().0,
        session.executed_instructions(),
        image.revision,
        image.dirty
    );
    image
}

/// Native persistence adapter for the 512-byte cartridge.
#[cfg(not(target_arch = "wasm32"))]
pub fn run_small(saved: Option<&[u8]>, press: bool, score: u16) -> SaveImage {
    run(SMALL_ROM, 512, saved, press, score)
}
/// Native persistence adapter for the 8-KiB cartridge.
#[cfg(not(target_arch = "wasm32"))]
pub fn run_large(saved: Option<&[u8]>, press: bool, score: u16) -> SaveImage {
    run(LARGE_ROM, 8192, saved, press, score)
}

/// Both sizes use automatic signature/command detection, then validated image
/// restore. Imports are dirty revisions; old completions cannot acknowledge them.
pub fn verify() {
    for (rom, capacity) in [(SMALL_ROM, 512), (LARGE_ROM, 8192)] {
        let image = run(rom, capacity, None, true, 1);
        assert!(image.dirty);
        assert!(!run(rom, capacity, Some(&image.bytes), false, 1).dirty);
        let next = run(rom, capacity, Some(&image.bytes), true, 2);
        assert!(next.dirty);
        let mut session = Session::new();
        session.load_rom(rom).unwrap();
        session.import_save(&next.bytes).unwrap();
        let old = session.save_image().unwrap();
        session.import_save(&image.bytes).unwrap();
        session.acknowledge_save(old.revision);
        assert!(session.save_image().unwrap().dirty);
        assert!(session.import_save(&image.bytes[..capacity - 1]).is_err());
        assert!(
            session
                .import_save(&vec![0; if capacity == 512 { 8192 } else { 512 }])
                .is_err()
        );
        assert_eq!(session.save_image().unwrap().bytes, image.bytes);
        // The explicit override is validated independently of signature evidence.
        session
            .load_rom_with_backup(
                rom,
                Some(if capacity == 512 {
                    BackupType::Eeprom512
                } else {
                    BackupType::Eeprom8k
                }),
            )
            .unwrap();
        assert!(
            session
                .load_save(&vec![0; if capacity == 512 { 8192 } else { 512 }])
                .is_err()
        );
        session.load_save(&image.bytes).unwrap();
    }
}

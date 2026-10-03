//! Bounded guest acceptance for Flash identification, score updates and recovery.
use gba_session::{BackupType, Button, FLASH64_BYTES, SaveImage, Session};

pub const ROM: &[u8] = include_bytes!("../flash-score.gba");

/// Check guest mailboxes and every completed pixel after three display frames.
/// A restored score must come from cartridge bytes loaded before execution.
pub fn run(saved: Option<&[u8]>, press: bool, score: u16) -> SaveImage {
    let mut session = Session::new();
    session.load_rom(ROM).unwrap();
    assert_eq!(
        session.backup_selection().selected(),
        Some(BackupType::Flash64)
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
    // Source-defined polling instructions, bounded independently of the decoder.
    assert!((0x08000148..=0x0800015c).contains(&pc) && pc % 4 == 0);
    assert!(session.cycles().0 <= 842750);
    assert_eq!(session.inspect16(0x03000000).unwrap(), 0x74);
    assert_eq!(session.inspect16(0x03000002).unwrap(), score);
    for (index, &pixel) in session.framebuffer().iter().enumerate() {
        let expected = if index / 240 < 8 && index % 240 < usize::from(score) * 8 {
            0x03e0
        } else {
            0
        };
        assert_eq!(pixel, expected, "Flash score framebuffer pixel {index}");
    }
    let image = session.save_image().unwrap();
    assert_eq!(image.bytes.len(), FLASH64_BYTES);
    assert_eq!(&image.bytes[..2], &[0xa5, score as u8]);
    assert!(image.bytes[2..].iter().all(|&byte| byte == 0xff));
    #[cfg(not(target_arch = "wasm32"))]
    println!(
        "Flash guest PASS: score={score} PC={pc:#010x} cycles={} instructions={} revision={} dirty={}",
        session.cycles().0,
        session.executed_instructions(),
        image.revision,
        image.dirty
    );
    image
}

/// A fresh session restores the score; the next increment must erase and
/// reprogram it because the transition from one to two requires setting a bit.
pub fn verify() {
    let image = run(None, true, 1);
    assert!(image.dirty);
    let restored = run(Some(&image.bytes), false, 1);
    assert!(!restored.dirty);
    let next = run(Some(&image.bytes), true, 2);
    assert!(next.dirty);
    let mut session = Session::new();
    session.load_rom(ROM).unwrap();
    session.import_save(&next.bytes).unwrap();
    let old = session.save_image().unwrap();
    session.import_save(&image.bytes).unwrap();
    session.acknowledge_save(old.revision);
    assert!(session.save_image().unwrap().dirty);
    assert!(session.import_save(&image.bytes[..32768]).is_err());
    assert_eq!(session.save_image().unwrap().bytes, image.bytes);
}

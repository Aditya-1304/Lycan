//! Deterministic guest acceptance shared by native storage and WASM verification.
use gba_session::{BackupType, Button, SaveImage, Session};

pub const ROM: &[u8] = include_bytes!("../sram.gba");

/// Executes the actual guest for three bounded display frames. Completion is
/// tied to the mailbox and the full completed framebuffer, never an idle loop.
pub fn run(saved: Option<&[u8]>, press: bool, score: u16) -> SaveImage {
    let mut machine = Session::new();
    machine.load_rom(ROM).unwrap();
    assert_eq!(
        machine.backup_selection().selected(),
        Some(BackupType::Sram)
    );
    if let Some(bytes) = saved {
        machine.load_save(bytes).unwrap();
    }
    machine.set_button(Button::A, press);
    let mut last_pc = 0;
    for _ in 0..3 {
        last_pc = machine
            .advance_frame_with_budget(200_000)
            .unwrap()
            .unwrap()
            .instruction_address;
    }
    // Both checkpoints are executed instructions in the declared VBlank polling
    // loop. The new chip has two initialization stores; restoration skips them.
    assert_eq!(
        last_pc,
        if saved.is_some() {
            0x08000110
        } else {
            0x08000118
        }
    );
    assert_eq!(machine.inspect16(0x03000000).unwrap(), 0x73);
    assert_eq!(machine.inspect16(0x03000002).unwrap(), score);
    assert!(machine.cycles().0 <= 842750);
    for (index, &pixel) in machine.framebuffer().iter().enumerate() {
        let expected = if index / 240 < 8 && index % 240 < usize::from(score) * 8 {
            0x03e0
        } else {
            0
        };
        assert_eq!(pixel, expected, "score framebuffer pixel {index}");
    }
    let image = machine.save_image().unwrap();
    assert_eq!(&image.bytes[..2], &[0xa5, score as u8]);
    println_evidence(&machine, &image, last_pc);
    image
}

#[cfg(not(target_arch = "wasm32"))]
fn println_evidence(machine: &Session, image: &SaveImage, last_pc: u32) {
    println!(
        "SRAM guest PASS: PC={:#010x}, cycles={}, instructions={}, revision={}, dirty={}",
        last_pc,
        machine.cycles().0,
        machine.executed_instructions(),
        image.revision,
        image.dirty
    );
}
#[cfg(target_arch = "wasm32")]
fn println_evidence(_: &Session, _: &SaveImage, _: u32) {}

/// Recovery, dirty coalescing, stale acknowledgements and rejected image sizes
/// exercise only observable cartridge state, not a particular host call sequence.
pub fn verify() {
    let image = run(None, true, 1);
    let restored = run(Some(&image.bytes), false, 1);
    assert!(!restored.dirty);
    let mut machine = Session::new();
    machine.load_rom(ROM).unwrap();
    machine.import_save(&image.bytes).unwrap();
    let old = machine.save_image().unwrap();
    let mut newer = image.bytes;
    newer[1] = 2;
    machine.import_save(&newer).unwrap();
    machine.acknowledge_save(old.revision);
    assert!(machine.save_image().unwrap().dirty);
    assert!(machine.import_save(&newer[..100]).is_err());
    assert_eq!(machine.save_image().unwrap().bytes, newer);
}

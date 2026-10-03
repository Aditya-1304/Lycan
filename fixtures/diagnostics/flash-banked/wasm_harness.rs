#[path = "contract.rs"]
mod contract;
#[unsafe(no_mangle)]
pub extern "C" fn verify() -> u32 {
    // Execute the same pinned upstream guest against the production WASM core.
    // The source defines r12 == 0 at this terminal instruction as success.
    let mut machine = gba_core::Machine::new();
    machine
        .load_rom_with_backup(
            include_bytes!("../gba-tests/save/flash128.gba"),
            Some(gba_core::BackupType::Flash128),
        )
        .unwrap();
    machine.enable_test_firmware();
    machine
        .run_until_pc(0x08000c4c, 4_000_000, gba_core::Cycle(50_000_000))
        .unwrap();
    assert_eq!(machine.registers()[12], 0);
    assert_eq!(machine.cpsr(), 0x600000df);
    contract::verify();
    1
}

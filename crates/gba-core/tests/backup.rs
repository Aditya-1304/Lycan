// The same cartridge contract is executed by the WASM harness.
#[path = "../../../roms/backup/contract.rs"]
mod contract;

#[test]
fn cartridge_load_selects_backup_hardware() {
    contract::verify();
}

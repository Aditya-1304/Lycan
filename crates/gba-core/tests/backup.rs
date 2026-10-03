// The same cartridge contract is executed by the WASM harness.
#[path = "../../../fixtures/diagnostics/backup/contract.rs"]
mod contract;

#[test]
fn cartridge_load_selects_backup_hardware() {
    contract::verify();
}

#[path = "contract.rs"]
mod contract;
#[unsafe(no_mangle)]
pub extern "C" fn verify() -> u32 {
    contract::verify();
    1
}

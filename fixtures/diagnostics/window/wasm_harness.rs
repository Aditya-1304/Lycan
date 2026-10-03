//! Executes the same guest/image oracle against the production WASM core.
#[path = "contract.rs"]
mod contract;
#[unsafe(no_mangle)]
pub extern "C" fn verify() -> u32 {
    contract::verify_with_capture(|_, _| {});
    1
}

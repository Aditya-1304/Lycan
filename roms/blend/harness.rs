//! Executes the same Slice-24 guest/image oracle against production WASM.

#[path = "contract.rs"]
mod contract;

#[unsafe(no_mangle)]
pub extern "C" fn verify() -> u32 {
    contract::verify_with_capture(|_, _| {});
    1
}

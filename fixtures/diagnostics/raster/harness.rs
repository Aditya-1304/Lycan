//! Production-WASM execution of the Slice-25 raster contract.

#[path = "contract.rs"]
mod contract;

#[unsafe(no_mangle)]
pub extern "C" fn verify() -> u32 {
    contract::verify_with_capture(|_, _| {});
    1
}

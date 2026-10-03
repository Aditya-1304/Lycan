//! Checks both original replay and upstream comparison on the production WASM core.
#[path = "contract.rs"]
mod contract;
#[unsafe(no_mangle)]
pub extern "C" fn verify() -> u32 {
    contract::verify_with_capture(|_, _| {});
    contract::verify_degenerate(|_, _| {});
    1
}

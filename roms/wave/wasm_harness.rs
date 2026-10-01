//! Runs the same guest/signal acceptance contract in the production WASM core.
#[path = "contract.rs"]
mod contract;
#[unsafe(no_mangle)]
pub extern "C" fn verify() -> u32 {
    contract::verify();
    1
}

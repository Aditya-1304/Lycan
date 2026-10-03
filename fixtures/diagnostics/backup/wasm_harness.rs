//! Executes the same load-time contract as the native regression.
mod contract;
#[unsafe(no_mangle)]
pub extern "C" fn verify() -> u32 {
    contract::verify();
    1
}

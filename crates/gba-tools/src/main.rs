#![forbid(unsafe_code)]

//! Headless GBA tools. Command behavior lives in the CLI and fixture modules.

mod benchmark;
mod cli;
mod fixtures;
mod probe;
mod support;

#[path = "../../../fixtures/diagnostics/affine/contract.rs"]
mod affine_contract;
#[path = "../../../fixtures/diagnostics/bios/contract.rs"]
mod bios_contract;
#[path = "../../../fixtures/diagnostics/blend/contract.rs"]
mod blend_contract;
#[path = "../../../fixtures/diagnostics/noise/contract.rs"]
mod noise_contract;
#[path = "../../../fixtures/diagnostics/affine-object/contract.rs"]
mod object_contract;
#[path = "../../../fixtures/diagnostics/pulse/contract.rs"]
mod pulse_contract;
#[path = "../../../fixtures/diagnostics/raster/contract.rs"]
mod raster_contract;
#[path = "../../../fixtures/diagnostics/rtc/contract.rs"]
mod rtc_contract;
#[path = "../../../fixtures/diagnostics/wave/contract.rs"]
mod wave_contract;
#[path = "../../../fixtures/diagnostics/window/contract.rs"]
mod window_contract;

fn main() {
    if let Err(error) = cli::run() {
        eprintln!("FAIL: {error}");
        std::process::exit(1);
    }
}

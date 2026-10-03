#![forbid(unsafe_code)]

mod backup;
mod rtc;
pub use rtc::RtcImage;
mod eeprom;
pub use eeprom::{EEPROM8K_BYTES, EEPROM512_BYTES};
#[cfg(feature = "eeprom-trace")]
pub use eeprom::{EepromTrace, EepromTraceEvent};
mod flash;
mod sram;
pub use backup::{BackupDetection, BackupSelection, BackupType, detect_backup};
pub use flash::{FLASH64_BYTES, FLASH128_BYTES};
pub use sram::{SRAM_BYTES, SaveImage, SaveStatus};

mod audio;
mod noise;
mod pulse;
mod wave;
pub use audio::PCM_RATE;

mod error;
mod input;
mod machine;
pub use error::{CoreError, RunError};
pub use input::{Button, ButtonState};
pub use machine::Machine;

/// Nominal ARM7TDMI clock rate used by the emulated hardware timeline.
pub const GBA_CLOCK_HZ: u64 = 16_777_216;
/// Number of hardware cycles in one GBA scanline.
pub const CYCLES_PER_SCANLINE: u64 = 1_232;
/// Total scanlines in one GBA frame, including vertical blanking.
pub const SCANLINES_PER_FRAME: u64 = 228;
/// Total hardware cycles in one complete GBA frame.
pub const CYCLES_PER_FRAME: u64 = CYCLES_PER_SCANLINE * SCANLINES_PER_FRAME;
/// Horizontal resolution of the GBA display.
pub const SCREEN_WIDTH: usize = 240;
/// Vertical resolution of the GBA display.
pub const SCREEN_HEIGHT: usize = 160;
/// Number of 16-bit pixels in one GBA framebuffer.
pub const FRAMEBUFFER_PIXELS: usize = SCREEN_WIDTH * SCREEN_HEIGHT;

/// Original division firmware, never a replacement for the retail BIOS.
pub const TEST_FIRMWARE: &[u8] =
    include_bytes!("../../../fixtures/diagnostics/test-firmware/division.bin");

/// Exact length of the supplied ARM7TDMI BIOS image.
pub const BIOS_SIZE: usize = 0x4000;

const ROM_START: u32 = 0x0800_0000;
const MAX_ROM_BYTES: usize = 32 * 1024 * 1024;
const EWRAM_START: u32 = 0x0200_0000;
const IWRAM_START: u32 = 0x0300_0000;
const IO_START: u32 = 0x0400_0000;
const IO_BYTES: usize = 0x400;
/// Undocumented write-only location touched by the retail BIOS during
/// RegisterRamReset. The written value has no modeled observable effect.
const BIOS_UNDOCUMENTED_IO_410: u32 = IO_START + 0x410;
const PALETTE_START: u32 = 0x0500_0000;
const PALETTE_BYTES: usize = 1024;
const VRAM_START: u32 = 0x0600_0000;
const VRAM_BYTES: usize = 96 * 1024;
const OAM_START: u32 = 0x0700_0000;
const OAM_BYTES: usize = 1024;
// The scanline renderer samples at 960 cycles, but the DISPSTAT HBlank flag
// follows the 1008-cycle mGBA timing model. IRQ edge timing is a later slice.
const HBLANK_FLAG_CYCLE: u64 = 1_008;

/// An absolute position on the emulated hardware cycle timeline.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Cycle(pub u64);

/// Actual cycles and last executed address from a bounded run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RunReport {
    pub instructions: usize,
    pub instruction_address: u32,
    pub cycles: Cycle,
}

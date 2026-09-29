//! Small guest programs used to exercise the core through real ARM instructions.

const ROM_HEADER_BYTES: usize = 0xC0;

// ARM program: configure mode 3/BG2, write eight BGR555 samples, then branch to itself.
// Each word is encoded explicitly so the fixture does not depend on a host assembler.
const PIXELS_PROGRAM: [u32; 27] = [
    0xE3A0_0640, // mov r0, #0x04000000 (DISPCNT)
    0xE3A0_1003, // mov r1, #3
    0xE381_1B01, // orr r1, r1, #0x400
    0xE1C0_10B0, // strh r1, [r0]
    0xE3A0_2660, // mov r2, #0x06000000 (VRAM)
    0xE3A0_301F, // mov r3, #0x001f (red)
    0xE1C2_30B0, // strh r3, [r2, #0]
    0xE3A0_3FF8, // mov r3, #0x03e0 (green)
    0xE1C2_30B2, // strh r3, [r2, #2]
    0xE3A0_3C7C, // mov r3, #0x7c00 (blue)
    0xE1C2_30B4, // strh r3, [r2, #4]
    0xE3A0_301F, // mov r3, #0x001f
    0xE383_3FF8, // orr r3, r3, #0x03e0 (yellow)
    0xE1C2_30B6, // strh r3, [r2, #6]
    0xE3A0_301F, // mov r3, #0x001f
    0xE383_3C7C, // orr r3, r3, #0x7c00 (magenta)
    0xE1C2_30B8, // strh r3, [r2, #8]
    0xE3A0_3FF8, // mov r3, #0x03e0
    0xE383_3C7C, // orr r3, r3, #0x7c00 (cyan)
    0xE1C2_30BA, // strh r3, [r2, #10]
    0xE3A0_301F, // mov r3, #0x001f
    0xE383_3FF8, // orr r3, r3, #0x03e0
    0xE383_3C7C, // orr r3, r3, #0x7c00 (white)
    0xE1C2_30BC, // strh r3, [r2, #12]
    0xE3A0_3000, // mov r3, #0 (black)
    0xE1C2_30BE, // strh r3, [r2, #14]
    0xEAFF_FFFE, // b .
];

/// Builds the controlled-startup `pixels.gba` fixture as cartridge bytes.
///
/// The entry branch skips the 192-byte cartridge header and transfers directly to
/// the guest program. The demo runner starts at the cartridge entry without a BIOS.
pub fn pixels_gba() -> Vec<u8> {
    let mut rom = vec![0; ROM_HEADER_BYTES];
    rom[0..4].copy_from_slice(&0xEA00_002Eu32.to_le_bytes());
    rom[0xA0..0xAC].copy_from_slice(b"PIXELS DEMO ");
    rom[0xAC..0xB0].copy_from_slice(b"PXLS");
    rom[0xB0..0xB2].copy_from_slice(b"01");
    rom[0xB2] = 0x96;

    let header_sum = rom[0xA0..=0xBC]
        .iter()
        .fold(0x19_u8, |sum, byte| sum.wrapping_add(*byte));
    rom[0xBD] = header_sum.wrapping_neg();

    for instruction in PIXELS_PROGRAM {
        rom.extend_from_slice(&instruction.to_le_bytes());
    }

    rom
}

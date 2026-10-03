//! Existing machine and bus regressions, retained without changing their assertions.

use super::cpu::{
    Access, AccessKind, CONTROLLED_START_CPSR, CPSR_F, CPSR_I, CPSR_SYSTEM_MODE, CPSR_T,
};
use super::display::{BLEND_LAYER_OBJ, ColorEffects, Display, LayerPixel, PixelStack, WindowMasks};

use super::*;
use crate::{
    BIOS_UNDOCUMENTED_IO_410, CYCLES_PER_FRAME, CYCLES_PER_SCANLINE, EWRAM_START,
    HBLANK_FLAG_CYCLE, IWRAM_START, OAM_BYTES, PALETTE_BYTES, ROM_START, SCREEN_WIDTH, VRAM_BYTES,
    VRAM_START,
};

/// Catches missing HBlank IF requests and HALT wakeups. Existing raster
/// tests exercise DMA requests, which do not require DISPSTAT's IRQ enable.
#[test]
fn hblank_irq_is_edge_triggered_and_wakes_halt() {
    let mut machine = Machine::new();
    machine
        .load_rom(include_bytes!(
            "../../../../fixtures/diagnostics/buttons.gba"
        ))
        .unwrap();
    let data = Access {
        kind: AccessKind::Data,
        sequential: false,
    };
    machine
        .system
        .write16_impl(IO_START + 4, 0x10, data)
        .unwrap();
    machine
        .system
        .write16_impl(IO_START + 0x200, 2, data)
        .unwrap();
    machine.system.halted = true;
    machine
        .advance_to(Cycle(HBLANK_FLAG_CYCLE - 1), 10)
        .unwrap();
    assert_eq!(machine.inspect16(IO_START + 0x202).unwrap() & 2, 0);
    machine.advance_to(Cycle(HBLANK_FLAG_CYCLE), 10).unwrap();
    assert_eq!(machine.inspect16(IO_START + 0x202).unwrap() & 2, 2);
    machine.step().unwrap();
    assert!(!machine.halted());
    machine
        .system
        .write16_impl(IO_START + 0x202, 2, data)
        .unwrap();
    machine.system.advance_time(20);
    assert_eq!(machine.inspect16(IO_START + 0x202).unwrap() & 2, 0);
    machine.system.advance_time(CYCLES_PER_SCANLINE);
    assert_eq!(machine.inspect16(IO_START + 0x202).unwrap() & 2, 2);
    machine.system.write16_impl(IO_START + 4, 0, data).unwrap();
    machine
        .system
        .write16_impl(IO_START + 0x202, 2, data)
        .unwrap();
    machine.system.advance_time(CYCLES_PER_SCANLINE);
    assert_eq!(machine.inspect16(IO_START + 0x202).unwrap() & 2, 0);
}

/// Catches truncating an unaligned OBJ's final mosaic block and anchoring
/// vertical groups to the object. Expected pixels follow mGBA's OBJ renderer.
#[test]
fn unaligned_object_mosaic_repeats_through_right_edge() {
    let mut bus = System::new();
    bus.display.control = 0x1040;
    for entry in bus.oam.as_chunks_mut::<8>().0 {
        entry[..2].copy_from_slice(&0x0200u16.to_le_bytes());
    }
    bus.oam[..2].copy_from_slice(&0x103du16.to_le_bytes()); // Y=61, mosaic.
    bus.oam[2..4].copy_from_slice(&101u16.to_le_bytes());
    bus.io[0x4d] = 0x22; // Three screen pixels in each axis.
    for row in 0..8 {
        bus.vram[0x10000 + row * 4..0x10004 + row * 4].fill((row as u8 + 1) * 0x11);
        bus.palette[0x202 + row * 2..0x204 + row * 2]
            .copy_from_slice(&(row as u16 + 1).to_le_bytes());
    }
    let masks = WindowMasks {
        layers: [0x3f; SCREEN_WIDTH],
    };
    for (line, color) in [(61, 1), (62, 1), (63, 3)] {
        bus.display.line = line;
        let mut stacks = [PixelStack::new(0); SCREEN_WIDTH];
        bus.display.render_objects(
            &bus.vram,
            &bus.palette,
            &bus.oam,
            &masks,
            &bus.io,
            &mut stacks,
        );
        let row: [u16; SCREEN_WIDTH] = std::array::from_fn(|x| stacks[x].top.color);
        assert_eq!(row[100], 0);
        assert!(
            row[101..111].iter().all(|pixel| *pixel == color),
            "line {line}: right-edge repetition or vertical phase is wrong: {:?}",
            &row[101..112]
        );
        assert_eq!(row[111], 0);
    }
}

/// Catches recomputing origins from line number after a mid-frame write,
/// and treating signed 28-bit reference coordinates as unsigned values.
#[test]
fn affine_reference_write_reloads_current_line_and_advances() {
    let mut bus = System::new();
    bus.display.control = 0x0403;
    bus.vram[0..2].copy_from_slice(&31u16.to_le_bytes());
    bus.vram[480..482].copy_from_slice(&992u16.to_le_bytes());
    bus.vram[960..962].copy_from_slice(&0x4210u16.to_le_bytes());
    bus.display
        .synchronize_to(960, &bus.vram, &bus.palette, &bus.io, &bus.oam);
    bus.write_halfword(0x0400002c, 0).unwrap();
    bus.write_halfword(0x0400002e, 0).unwrap();
    bus.display
        .synchronize_to(2192, &bus.vram, &bus.palette, &bus.io, &bus.oam);
    assert_eq!(bus.display.drawing[240], 31);
    bus.display
        .synchronize_to(3424, &bus.vram, &bus.palette, &bus.io, &bus.oam);
    assert_eq!(bus.display.drawing[480], 992);
    bus.write_halfword(0x04000028, 0xff00).unwrap();
    bus.write_halfword(0x0400002a, 0x0fff).unwrap();
    bus.display
        .synchronize_to(4656, &bus.vram, &bus.palette, &bus.io, &bus.oam);
    assert_eq!(bus.display.drawing[720], 0);
    assert_eq!(bus.display.drawing[721], 0x4210); // X=-1 clips, then X=0 samples row 2.
}

/// Catches screen-linear sampling, missing mode-5 pages, and incorrect clipping.
#[test]
fn affine_bitmap_rotation_clips_and_selects_page() {
    let mut display = Display::new();
    let mut io = vec![0; IO_BYTES];
    let mut vram = vec![0; VRAM_BYTES];
    let palette = vec![0; PALETTE_BYTES];
    let oam = vec![0; OAM_BYTES];
    io[0x24..0x26].copy_from_slice(&256i16.to_le_bytes());
    io[0x28..0x2c].copy_from_slice(&256i32.to_le_bytes());
    vram[0xa002..0xa004].copy_from_slice(&0x1234u16.to_le_bytes());
    vram[0xa142..0xa144].copy_from_slice(&0x5678u16.to_le_bytes());
    display.control = 0x0415;
    display.synchronize_to(960, &vram, &palette, &io, &oam);
    assert_eq!(display.drawing[0], 0x1234);
    assert_eq!(display.drawing[1], 0x5678);
    assert_eq!(display.drawing[128], 0);
}

#[test]
fn retail_bios_undocumented_0410_byte_write_is_accepted() {
    let mut bus = System::new();
    let data = Access {
        kind: AccessKind::Data,
        sequential: false,
    };
    let before = bus.cycles;

    bus.write8(BIOS_UNDOCUMENTED_IO_410, 0xff, data)
        .expect("retail BIOS 0x04000410 byte write must be accepted");

    // The write still occupies a real bus access.
    assert!(bus.cycles > before);

    // Keep neighboring unused addresses outside the ordinary I/O bank.
    assert!(matches!(
        bus.write8(BIOS_UNDOCUMENTED_IO_410 + 1, 0xff, data),
        Err(CoreError::UnmappedAddress { address, width: 1 })
            if address == BIOS_UNDOCUMENTED_IO_410 + 1
    ));
}

/// DMA command lengths select the EEPROM address width. A high block must
/// survive serial readback and restore without aliasing a smaller device.
#[test]
fn eeprom_dma_detects_capacity_and_restores_serial_blocks() {
    let access = Access {
        kind: AccessKind::Data,
        sequential: false,
    };
    for (address_bits, block, capacity) in [(6, 63, 512), (14, 1023, 8192)] {
        let mut machine = Machine::new();
        machine
            .load_rom(include_bytes!(
                "../../../../fixtures/diagnostics/backup/eeprom.gba"
            ))
            .unwrap();
        let data = 0xa501_2345_6789_abcd_u64;
        let transfer = |machine: &mut Machine, bits: &[u16]| {
            for (index, bit) in bits.iter().enumerate() {
                machine
                    .system
                    .write16_impl(0x02000000 + index as u32 * 2, *bit, access)
                    .unwrap();
            }
            for (address, value) in [(0x040000d4, 0x02000000), (0x040000d8, 0x0d000000)] {
                machine.system.write32_impl(address, value, access).unwrap();
            }
            machine
                .system
                .write16_impl(0x040000dc, bits.len() as u16, access)
                .unwrap();
            machine
                .system
                .write16_impl(0x040000de, 0x8000, access)
                .unwrap();
            while machine.system.dma[3].active {
                machine.system.dma_beat(3).unwrap();
            }
        };
        let address: Vec<u16> = (0..address_bits)
            .rev()
            .map(|shift| ((block >> shift) & 1) as u16)
            .collect();
        let mut write = vec![1, 0];
        write.extend_from_slice(&address);
        write.extend((0..64).rev().map(|shift| ((data >> shift) & 1) as u16));
        write.push(0);
        transfer(&mut machine, &write);
        let saved = machine
            .save_image()
            .expect("EEPROM must expose persistence");
        assert_eq!(saved.bytes.len(), capacity);
        assert_eq!(&saved.bytes[block * 8..block * 8 + 8], &data.to_be_bytes());
        assert!(saved.dirty);
        machine.load_save(&saved.bytes).unwrap();
        machine.reset();
        let mut read = vec![1, 1];
        read.extend_from_slice(&address);
        read.push(0);
        transfer(&mut machine, &read);
        for _ in 0..4 {
            machine.system.read16_impl(0x0d000000, access).unwrap();
        }
        let mut actual = 0_u64;
        for _ in 0..64 {
            actual = (actual << 1)
                | u64::from(machine.system.read16_impl(0x0d000000, access).unwrap() & 1);
        }
        assert_eq!(actual, data);
        assert!(!machine.save_image().unwrap().dirty);
        assert!(
            machine
                .import_save(&vec![0; if capacity == 512 { 8192 } else { 512 }])
                .is_err()
        );
        assert_eq!(machine.save_image().unwrap().bytes, saved.bytes);
    }
}

/// Bank-local programming and sector erase must preserve the other bank.
/// A complete persisted image restores both banks after protocol reset.
#[test]
fn flash_banks_preserve_and_restore_independent_bytes() {
    let mut machine = Machine::new();
    machine
        .load_rom(include_bytes!(
            "../../../../fixtures/diagnostics/backup/flash128.gba"
        ))
        .unwrap();
    let access = Access {
        kind: AccessKind::Data,
        sequential: false,
    };
    let mut command = |bank, value| {
        for (offset, byte) in [
            (0x5555, 0xaa),
            (0x2aaa, 0x55),
            (0x5555, 0xb0),
            (0, bank),
            (0x5555, 0xaa),
            (0x2aaa, 0x55),
            (0x5555, 0xa0),
            (0x123, value),
        ] {
            machine
                .system
                .write8(0x0e000000 + offset, byte, access)
                .unwrap();
        }
    };
    command(0, 0x42);
    command(1, 0x24);
    let saved = machine
        .save_image()
        .expect("Flash128 must expose persistence");
    assert_eq!(saved.bytes.len(), 131072);
    assert_eq!(saved.bytes[0x123], 0x42);
    assert_eq!(saved.bytes[0x10123], 0x24);
    for (offset, byte) in [
        (0x5555, 0xaa),
        (0x2aaa, 0x55),
        (0x5555, 0x80),
        (0x5555, 0xaa),
        (0x2aaa, 0x55),
        (0, 0x30),
    ] {
        machine
            .system
            .write8(0x0e000000 + offset, byte, access)
            .unwrap();
    }
    let erased = machine.save_image().unwrap();
    assert_eq!(erased.bytes[0x123], 0x42);
    assert_eq!(erased.bytes[0x10123], 0xff);
    machine.load_save(&saved.bytes).unwrap();
    machine.reset();
    assert_eq!(machine.system.read8(0x0e000123, access).unwrap(), 0x42);
    assert_eq!(machine.save_image().unwrap().bytes, saved.bytes);
    assert!(machine.import_save(&saved.bytes[..65536]).is_err());
    assert_eq!(machine.save_image().unwrap().bytes, saved.bytes);
}

/// A save read interrupts an unfinished Flash unlock. Resuming its remaining
/// writes must not program a byte; a fresh, uninterrupted sequence must work.
#[test]
fn flash_read_cancels_partial_unlock() {
    let mut machine = Machine::new();
    machine
        .load_rom(include_bytes!(
            "../../../../fixtures/diagnostics/backup/flash64.gba"
        ))
        .unwrap();
    let access = Access {
        kind: AccessKind::Data,
        sequential: false,
    };
    machine.system.write8(0x0e005555, 0xaa, access).unwrap();
    assert_eq!(machine.system.read8(0x0e000123, access).unwrap(), 0xff);
    for (offset, value) in [(0x2aaa, 0x55), (0x5555, 0xa0), (0x123, 0x42)] {
        machine
            .system
            .write8(0x0e000000 + offset, value, access)
            .unwrap();
    }
    assert_eq!(machine.system.read8(0x0e000123, access).unwrap(), 0xff);
    assert!(!machine.save_image().unwrap().dirty);
    for (offset, value) in [
        (0x5555, 0xaa),
        (0x2aaa, 0x55),
        (0x5555, 0xa0),
        (0x123, 0x42),
    ] {
        machine
            .system
            .write8(0x0e000000 + offset, value, access)
            .unwrap();
    }
    assert_eq!(machine.system.read8(0x0e000123, access).unwrap(), 0x42);
    machine.reset();
    assert_eq!(machine.system.read8(0x0e000123, access).unwrap(), 0x42);
    assert!(machine.save_image().unwrap().dirty);
}

/// A detected SRAM cartridge must expose mirrored byte storage rather than
/// returning unmapped-bus errors or losing the score across a CPU reset.
#[test]
fn sram_byte_bus_survives_reset() {
    let mut machine = Machine::new();
    machine
        .load_rom(include_bytes!(
            "../../../../fixtures/diagnostics/backup/sram.gba"
        ))
        .unwrap();
    let access = Access {
        kind: AccessKind::Data,
        sequential: false,
    };
    assert_eq!(machine.system.read8(0x0e000000, access).unwrap(), 0xff);
    machine.system.write8(0x0e000003, 42, access).unwrap();
    assert_eq!(machine.system.read8(0x0f008003, access).unwrap(), 42);
    machine.reset();
    assert_eq!(machine.system.read8(0x0e000003, access).unwrap(), 42);
}

// A block load crossing from mapped MMIO into an unmapped address must
// retain CPU pipeline context, rather than expose the preceding MMIO value.
// Existing timing tests do not cross a mapped/unmapped boundary in one LDM.
#[test]
fn open_bus_block_load_retains_cpu_prefetch() {
    let words = [0xe8900006u32, 0xe1a00000, 0xe1a04004, 0xeafffffe];
    let rom: Vec<u8> = words.iter().flat_map(|word| word.to_le_bytes()).collect();
    let mut machine = Machine::new();
    machine.load_rom(&rom).unwrap();
    machine.cpu.registers[0] = IO_START + 0x3fc;
    machine.step().unwrap();
    assert_eq!(machine.registers()[1], 0);
    assert_eq!(machine.registers()[2], 0xe1a04004);
}

// Existing controlled fixtures cannot catch commercial boot bypassing the
// reset vector or losing cartridge storage when the BIOS is retained.
#[test]
fn supplied_bios_boot_and_reset_preserve_backup() {
    let mut machine = Machine::new();
    assert!(
        machine
            .boot_rom_with_bios(&[0; 8], Some(BackupType::Sram))
            .is_err()
    );
    assert!(machine.load_bios(&[0; 4]).is_err());
    let mut bios = vec![0; BIOS_SIZE];
    bios[..4].copy_from_slice(&0xeafffffeu32.to_le_bytes());
    machine.load_bios(&bios).unwrap();
    machine
        .boot_rom_with_bios(&[0; 8], Some(BackupType::Sram))
        .unwrap();
    assert_eq!(machine.registers()[15], 0);
    assert_eq!(machine.registers()[13], 0);
    assert_eq!(machine.cpsr(), 0xd3);
    machine.system.backup_write(0x0e000000, 0x42);
    let saved = machine.save_image().unwrap();
    machine.step().unwrap();
    machine.reset();
    assert_eq!(machine.registers()[15], 0);
    assert_eq!(machine.save_image().unwrap().bytes, saved.bytes);
    assert_eq!(machine.save_image().unwrap().revision, saved.revision);
    machine.step().unwrap();
    machine.load_rom(&[0; 8]).unwrap();
    assert_eq!(machine.registers()[15], ROM_START);
}

// Protected reads must retain the last fetched BIOS word after a redirect;
// exposing raw BIOS data would fail upstream startup/SWI/IRQ checks.
#[test]
fn bios_protection_uses_execution_context_and_fetch_latch() {
    let mut machine = Machine::new();
    let mut bios = vec![0; BIOS_SIZE];
    bios[..4].copy_from_slice(&0x12345678u32.to_le_bytes());
    bios[4..8].copy_from_slice(&0xabcdef01u32.to_le_bytes());
    machine.load_bios(&bios).unwrap();
    machine.system.bios_enabled = true;
    machine.system.execution_address = 0;
    let fetch = Access {
        kind: AccessKind::Fetch,
        sequential: false,
    };
    let data = Access {
        kind: AccessKind::Data,
        sequential: false,
    };
    assert_eq!(machine.system.read32(4, fetch).unwrap(), 0xabcdef01);
    assert_eq!(machine.system.read32(0, data).unwrap(), 0x12345678);
    machine.system.execution_address = ROM_START;
    assert_eq!(machine.system.read32(0, data).unwrap(), 0xabcdef01);
    assert_eq!(machine.system.read16(2, data).unwrap(), 0xabcd);
    assert_eq!(machine.system.read8(1, data).unwrap(), 0xef);
}

/// Exercises sleep, firmware dispatch, W1C and masked wake through guest code.
#[test]
fn vblank_guest_sleeps_dispatches_and_acknowledges() {
    let rom = include_bytes!("../../../../fixtures/diagnostics/vblank.gba");
    let mut machine = Machine::new();
    machine.load_rom(rom).unwrap();
    machine.enable_test_firmware();
    machine
        .advance_to(Cycle(4 * CYCLES_PER_FRAME), 20_000)
        .unwrap();
    assert_eq!(machine.inspect16(IWRAM_START + 2).unwrap(), 4);
    assert_eq!(machine.inspect16(IWRAM_START + 4).unwrap(), 1);
    assert_eq!(machine.inspect16(IWRAM_START + 6).unwrap(), 0);
    assert_eq!(machine.inspect16(IWRAM_START + 8).unwrap(), 4);
    assert_eq!(machine.cpsr() & 0xff, 0x5f);
    assert!(machine.executed_instructions() < 1000);
    let mut thumb = rom.to_vec();
    thumb[0x300..0x304].copy_from_slice(&15u32.to_le_bytes());
    machine.load_rom(&thumb).unwrap();
    machine.enable_test_firmware();
    machine
        .advance_to(Cycle(4 * CYCLES_PER_FRAME), 20_000)
        .unwrap();
    assert_eq!(machine.inspect16(IWRAM_START + 2).unwrap(), 4);
    assert!(machine.is_thumb());
    for (configuration, wakes) in [(6u32, 0), (5, 2), (3, 2)] {
        let mut bytes = rom.to_vec();
        bytes[0x300..0x304].copy_from_slice(&configuration.to_le_bytes());
        machine.load_rom(&bytes).unwrap();
        machine.enable_test_firmware();
        machine
            .advance_to(Cycle(2 * CYCLES_PER_FRAME), 20_000)
            .unwrap();
        assert_eq!(machine.inspect16(IWRAM_START + 2).unwrap(), 0);
        assert_eq!(machine.inspect16(IWRAM_START + 8).unwrap(), wakes);
    }
    machine.load_rom(rom).unwrap();
    assert!(matches!(
        machine.advance_to(Cycle(CYCLES_PER_FRAME), 20_000),
        Err(RunError::Core(CoreError::UnmappedAddress {
            address: 0x18,
            width: 4
        }))
    ));
    assert_eq!(machine.inspect16(IWRAM_START + 2).unwrap(), 0);
}

// Catches ignored WAITCNT writes and ROM aliases charged as ordinary memory.
// Existing fixtures use default WS0 timing exclusively.
#[test]
fn cartridge_windows_use_programmed_waitstates() {
    let mut bus = System::new();
    bus.rom = vec![0x34, 0x12, 0x78, 0x56];
    let data = Access {
        kind: AccessKind::Data,
        sequential: false,
    };
    bus.rom.resize(0x20004, 0);
    for (bank, address) in [0x08000000, 0x0a000000, 0x0c000000].into_iter().enumerate() {
        for (selector, first) in [5, 4, 3, 9].into_iter().enumerate() {
            for (fast, second) in [[3, 5, 9][bank], 2].into_iter().enumerate() {
                let control =
                    ((selector as u16) << [2, 5, 8][bank]) | ((fast as u16) << [4, 7, 10][bank]);
                bus.write16_impl(IO_START + 0x204, control, data).unwrap();
                let start = bus.cycles;
                assert_eq!(bus.read32_impl(address, data).unwrap(), 0x56781234);
                assert_eq!(bus.cycles - start, first + second);
                let start = bus.cycles;
                bus.read16_impl(
                    address + 4,
                    Access {
                        sequential: true,
                        ..data
                    },
                )
                .unwrap();
                assert_eq!(bus.cycles - start, second);
                // The cartridge forces N timing at each 128 KiB boundary.
                bus.read16_impl(address + 0x1fffe, data).unwrap();
                let start = bus.cycles;
                bus.read16_impl(
                    address + 0x20000,
                    Access {
                        sequential: true,
                        ..data
                    },
                )
                .unwrap();
                assert_eq!(bus.cycles - start, first);
            }
        }
    }
    for (selector, cycles) in [5, 4, 3, 9].into_iter().enumerate() {
        bus.write16_impl(IO_START + 0x204, selector as u16, data)
            .unwrap();
        let start = bus.cycles;
        bus.charge(0x0e000000, 1, data);
        assert_eq!(bus.cycles - start, cycles);
    }
}

// Catches lost partial fills, incorrect word consumption, and stale opcodes
// surviving a cartridge data access. No earlier test enables prefetch.
#[test]
fn prefetch_consumes_halfwords_and_resets_on_cartridge_data() {
    let mut bus = System::new();
    let fetch = Access {
        kind: AccessKind::Fetch,
        sequential: true,
    };
    let data = Access {
        kind: AccessKind::Data,
        sequential: false,
    };
    bus.write16_impl(IO_START + 0x204, 0x4000, data).unwrap();
    bus.charge(
        ROM_START,
        2,
        Access {
            sequential: false,
            ..fetch
        },
    );
    bus.idle(2);
    let start = bus.cycles;
    bus.charge(ROM_START + 2, 2, fetch);
    assert_eq!(bus.cycles - start, 1);
    bus.idle(24);
    let start = bus.cycles;
    bus.charge(ROM_START + 4, 4, fetch);
    assert_eq!(bus.cycles - start, 1);
    bus.charge(ROM_START + 0x100, 2, data);
    let start = bus.cycles;
    bus.charge(ROM_START + 8, 2, fetch);
    assert_eq!(bus.cycles - start, 5);
    bus.write16_impl(IO_START + 0x204, 0, data).unwrap();
    bus.idle(24);
    let start = bus.cycles;
    bus.charge(ROM_START + 10, 2, fetch);
    assert_eq!(bus.cycles - start, 3);
}

// Catches stale ARM pipeline words after BX, incorrect Thumb PC alignment,
// and a BL return that loses its Thumb tag before the final ARM return.
// Earlier guest fixtures execute exclusively in ARM state.
#[test]
fn counter_returns_from_thumb_with_architectural_pc_values() {
    let mut machine = Machine::new();
    machine
        .load_rom(include_bytes!(
            "../../../../fixtures/diagnostics/counter.gba"
        ))
        .unwrap();
    machine.set_button(Button::A, true);
    let report = machine
        .run_until_pc(0x08000040, 100_000, Cycle(300_000))
        .unwrap();
    assert_eq!(report.instruction_address, 0x08000040);
    assert_eq!(machine.cpu.cpsr & CPSR_T, 0);
    assert_eq!(machine.cpu.registers[15], 0x08000044);
    assert_eq!(machine.cpu.registers[0], 1);
    assert_eq!(machine.cpu.registers[2], 0x080000d0);
    assert_eq!(machine.cpu.registers[4], 0x080000b4);
    assert_eq!(machine.cpu.registers[14], 0x080000bf);
}

// Catches a guest seeing pressed keys at reset or writable hardware status.
// Existing tests exercise VRAM and CPU startup, not hardware input reads.
#[test]
fn keypad_is_active_low_and_hardware_status_is_read_only() {
    let mut machine = Machine::new();
    assert_eq!(machine.inspect16(IO_START + 0x130).unwrap(), 0x03ff);
    let access = Access {
        kind: AccessKind::Data,
        sequential: false,
    };
    machine
        .system
        .write16_impl(IO_START + 0x130, 0, access)
        .unwrap();
    machine
        .system
        .write16_impl(IO_START + 6, 99, access)
        .unwrap();
    assert_eq!(machine.inspect16(IO_START + 0x130).unwrap(), 0x03ff);
    assert_eq!(machine.inspect16(IO_START + 6).unwrap(), 0);
}

// Catches VBlank polling loops hanging, HBlank flags staying high, and
// VCOUNT comparisons using frontend redraws instead of the hardware clock.
#[test]
fn display_status_tracks_line_blank_and_compare_boundaries() {
    let mut machine = Machine::new();
    let access = Access {
        kind: AccessKind::Data,
        sequential: false,
    };
    machine
        .system
        .write16_impl(IO_START + 4, (160 << 8) | 0x38, access)
        .unwrap();
    for (cycle, line, flags) in [
        (960, 0, 0),
        (1007, 0, 0),
        (1008, 0, 2),
        (1232, 1, 0),
        (160 * 1232, 160, 5),
        (227 * 1232, 227, 0),
        (CYCLES_PER_FRAME, 0, 0),
    ] {
        machine.system.advance_time(cycle - machine.cycles().0);
        assert_eq!(machine.inspect16(IO_START + 6).unwrap(), line);
        assert_eq!(
            machine.inspect16(IO_START + 4).unwrap(),
            (160 << 8) | 0x38 | flags
        );
    }
}

#[test]
fn vcount_irq_latches_once_per_compare_edge() {
    let mut machine = Machine::new();

    let access = Access {
        kind: AccessKind::Data,
        sequential: false,
    };

    // VCOUNT compare = 150, VCOUNT IRQ enabled.
    machine
        .system
        .write16_impl(IO_START + 4, (150 << 8) | 0x20, access)
        .unwrap();

    // Enable VCOUNT in IE.
    machine
        .system
        .write16_impl(IO_START + 0x200, 1 << 2, access)
        .unwrap();

    let first_edge = 150 * CYCLES_PER_SCANLINE;

    assert!(machine.cycles().0 < first_edge);

    // Stop immediately before the comparison edge.
    machine
        .system
        .advance_time(first_edge - machine.cycles().0 - 1);

    assert_eq!(machine.inspect16(IO_START + 0x202).unwrap() & (1 << 2), 0,);

    // Enter scanline 150.
    machine.system.advance_time(1);

    assert_eq!(machine.inspect16(IO_START + 6).unwrap(), 150,);

    assert_ne!(
        machine.inspect16(IO_START + 4).unwrap() & 0x04,
        0,
        "VCOUNT comparison status must be set",
    );

    assert_eq!(
        machine.inspect16(IO_START + 0x202).unwrap() & (1 << 2),
        1 << 2,
        "VCOUNT must latch IF bit 2",
    );

    // Guest acknowledges VCOUNT.
    machine
        .system
        .write16_impl(IO_START + 0x202, 1 << 2, access)
        .unwrap();

    assert_eq!(machine.inspect16(IO_START + 0x202).unwrap() & (1 << 2), 0,);

    // Remaining inside line 150 must NOT continuously regenerate the IRQ.
    machine.system.advance_time(100);

    assert_eq!(
        machine.inspect16(IO_START + 0x202).unwrap() & (1 << 2),
        0,
        "VCOUNT IRQ must trigger only on the compare edge",
    );

    // It should fire again on line 150 of the following frame.
    let second_edge = CYCLES_PER_FRAME + first_edge;

    machine
        .system
        .advance_time(second_edge - machine.cycles().0);

    assert_eq!(
        machine.inspect16(IO_START + 0x202).unwrap() & (1 << 2),
        1 << 2,
    );
}

#[test]
fn guest_stores_are_published_only_after_display_scanout() {
    let mut machine = Machine::new();
    let rom: Vec<u8> = [
        0xE3A00640u32,
        0xE3A01003,
        0xE3811B01,
        0xE1C010B0,
        0xE3A02660,
        0xE3A0301F,
        0xE1C230B0,
        0xEAFFFFFE,
        0,
        0,
    ]
    .into_iter()
    .flat_map(u32::to_le_bytes)
    .collect();

    machine.load_rom(&rom).unwrap();
    let report = machine
        .run_until_pc(ROM_START + 28, 64, Cycle(1000))
        .unwrap();

    assert_eq!(report.instructions, 8);
    // A VRAM write is not a completed frame. Presentation must wait for scanout.
    assert_eq!(machine.framebuffer()[0], 0);
    assert_eq!(machine.framebuffer_generation(), 0);
    let report = machine
        .advance_to(Cycle(CYCLES_PER_FRAME), 100_000)
        .unwrap();
    assert!(report.cycles.0 >= CYCLES_PER_FRAME);
    assert_eq!(machine.framebuffer_generation(), 1);
    assert_eq!(machine.framebuffer()[0], 0x001F);
    // A guest that keeps looping still respects a caller's finite work budget.
    assert!(matches!(
        machine.advance_to(Cycle(2 * CYCLES_PER_FRAME), 1),
        Err(RunError::StepLimitExceeded { .. })
    ));
}

// Catches missing FIFO refill, DMA source rewind and sample production tied
// to host deadlines. Existing DMA3 image tests do not consume audio FIFOs.
#[test]
fn timer_fifo_dma_sound_is_identical_across_bounded_advances() {
    let run = |chunk: u64, fifo: u32, bias: u16, channel: usize| {
        let mut machine = Machine::new();
        machine.system.halted = true;
        for (index, byte) in machine.system.ewram.iter_mut().take(128).enumerate() {
            *byte = (index as u8).wrapping_mul(3);
        }
        let data = Access {
            kind: AccessKind::Data,
            sequential: false,
        };
        machine
            .system
            .write32_impl(IO_START + 0xb0 + channel as u32 * 12, EWRAM_START, data)
            .unwrap();
        machine
            .system
            .write32_impl(IO_START + 0xb4 + channel as u32 * 12, IO_START + fifo, data)
            .unwrap();
        // FIFO mode forces four words and a fixed destination regardless of count/width.
        machine
            .system
            .write32_impl(IO_START + 0xb8 + channel as u32 * 12, 0xb2000001, data)
            .unwrap();
        machine
            .system
            .write16_impl(IO_START + 0x100, 0xfe00, data)
            .unwrap();
        machine
            .system
            .write16_impl(IO_START + 0x102, 0xc0, data)
            .unwrap();
        machine
            .system
            .write16_impl(IO_START + 0x84, 0x80, data)
            .unwrap();
        machine
            .system
            .write16_impl(
                IO_START + 0x82,
                if fifo == 0xa0 { 0x304 } else { 0x3008 },
                data,
            )
            .unwrap();
        machine
            .system
            .write16_impl(IO_START + 0x88, bias, data)
            .unwrap();
        let end = machine.cycles().0 + 48 * 512;
        let mut pcm = Vec::new();
        while machine.cycles().0 < end {
            machine
                .advance_to(Cycle((machine.cycles().0 + chunk).min(end)), 1)
                .unwrap();
            machine.drain_pcm(&mut pcm);
        }
        assert_eq!(machine.inspect16(IO_START + 0x202).unwrap() & 8, 8);
        assert_eq!(pcm.len(), end as usize / 512);
        assert!(pcm.iter().any(|&sample| sample > 0.5));
        assert!(pcm.iter().any(|&sample| sample < -0.5));
        pcm
    };
    // DMA2 previously never enabled or refilled. Both channels, FIFO
    // destinations and PWM cadences must remain independent of
    // host chunk size, including chunks that split a PWM sample interval.
    for fifo in [0xa0, 0xa4] {
        for bias in [0x200, 0x4200, 0x8200, 0xc200] {
            for channel in [1, 2] {
                assert_eq!(run(37, fifo, bias, channel), run(4096, fifo, bias, channel));
            }
        }
    }
}

#[test]
fn dma_upload_resumes_at_deadlines_and_latches_vblank_irq() {
    // A large immediate upload must relinquish the host at a beat boundary,
    // without retiring CPU instructions or skipping the intervening IRQ edge.
    let mut machine = Machine::new();
    machine.system.halted = true;
    machine.system.ewram.fill(0x5a);
    let data = Access {
        kind: AccessKind::Data,
        sequential: false,
    };
    machine.system.write16_impl(IO_START + 4, 8, data).unwrap();
    machine
        .system
        .advance_time(160 * CYCLES_PER_SCANLINE - 20 - machine.cycles().0);
    machine
        .system
        .write32_impl(IO_START + 0xd4, EWRAM_START, data)
        .unwrap();
    machine
        .system
        .write32_impl(IO_START + 0xd8, VRAM_START, data)
        .unwrap();
    machine
        .system
        .write32_impl(IO_START + 0xdc, 0x8000_4000, data)
        .unwrap();
    let start = machine.cycles().0;
    machine.advance_to(Cycle(start + 8), 1).unwrap();
    assert_eq!(machine.inspect16(VRAM_START).unwrap(), 0x5a5a);
    assert_eq!(machine.inspect16(VRAM_START + 32766).unwrap(), 0);
    assert_eq!(machine.inspect16(IO_START + 0xde).unwrap() & 0x8000, 0x8000);
    assert_eq!(machine.executed_instructions(), 0);
    assert!(machine.cycles().0 <= start + 12);
    machine.advance_to(Cycle(start + 70000), 1).unwrap();
    assert_eq!(machine.inspect16(VRAM_START + 32766).unwrap(), 0x5a5a);
    assert_eq!(machine.inspect16(IO_START + 0xde).unwrap() & 0x8000, 0);
    assert_eq!(machine.inspect16(IO_START + 0x202).unwrap() & 1, 1);
}

#[test]
fn repeating_dma_reloads_destination_without_rewinding_source() {
    // Repeated HBlank uploads must progress through successive source rows.
    // Reloading both addresses would silently display the first row forever.
    let mut machine = Machine::new();
    machine.system.halted = true;
    let data = Access {
        kind: AccessKind::Data,
        sequential: false,
    };
    for (offset, value) in [(0, 0x11223344), (4, 0x55667788)] {
        machine
            .system
            .write32_impl(EWRAM_START + offset, value, data)
            .unwrap();
    }
    machine
        .system
        .write32_impl(IO_START + 0xd4, EWRAM_START, data)
        .unwrap();
    machine
        .system
        .write32_impl(IO_START + 0xd8, VRAM_START, data)
        .unwrap();
    machine
        .system
        .write32_impl(IO_START + 0xdc, 0xa6600001, data)
        .unwrap();
    machine.advance_to(Cycle(HBLANK_FLAG_CYCLE - 1), 1).unwrap();
    assert_eq!(machine.inspect16(VRAM_START).unwrap(), 0);
    machine
        .advance_to(Cycle(HBLANK_FLAG_CYCLE + 20), 1)
        .unwrap();
    assert_eq!(machine.inspect16(VRAM_START).unwrap(), 0x3344);
    assert_eq!(machine.inspect16(VRAM_START + 2).unwrap(), 0x1122);
    machine
        .advance_to(Cycle(CYCLES_PER_SCANLINE + HBLANK_FLAG_CYCLE + 20), 1)
        .unwrap();
    assert_eq!(machine.inspect16(VRAM_START).unwrap(), 0x7788);
    assert_eq!(machine.inspect16(VRAM_START + 2).unwrap(), 0x5566);
    assert_eq!(machine.inspect16(VRAM_START + 4).unwrap(), 0);
    machine
        .system
        .write16_impl(IO_START + 0xde, 0, data)
        .unwrap();
    machine
        .advance_to(Cycle(3 * CYCLES_PER_SCANLINE), 1)
        .unwrap();
    assert_eq!(machine.inspect16(VRAM_START).unwrap(), 0x7788);
}

#[test]
fn dma0_hblank_repeat_reloads_destination_and_skips_vblank() {
    let mut machine = Machine::new();

    // HALT isolates scheduler wakeups from CPU instruction execution.
    machine.system.halted = true;

    // Each visible HBlank consumes one four-byte pair of register values.
    for index in 0..160usize {
        let value = u16::try_from(index).unwrap();
        let word = u32::from(0x0404u16) | (u32::from(value) << 16);
        let offset = index * 4;
        machine.system.ewram[offset..offset + 4].copy_from_slice(&word.to_le_bytes());
    }

    let data = Access {
        kind: AccessKind::Data,
        sequential: false,
    };
    machine
        .system
        .write32_impl(IO_START + 0xb0, EWRAM_START, data)
        .unwrap();
    machine
        .system
        .write32_impl(IO_START + 0xb4, IO_START + 0x52, data)
        .unwrap();
    // Count=2 halfwords; HBlank timing, repeat, and destination reload.
    machine
        .system
        .write32_impl(IO_START + 0xb8, (u32::from(0xa260u16) << 16) | 2, data)
        .unwrap();

    machine
        .advance_to(Cycle(HBLANK_FLAG_CYCLE + 32), 1)
        .unwrap();

    assert_eq!(machine.inspect16(IO_START + 0x52).unwrap(), 0x0404);
    assert_eq!(machine.inspect16(IO_START + 0x54).unwrap(), 0);
    assert_eq!(machine.system.dma[0].source, EWRAM_START + 4);
    assert_eq!(machine.system.dma[0].destination, IO_START + 0x52);
    assert_eq!(machine.executed_instructions(), 0);

    machine
        .advance_to(Cycle(CYCLES_PER_SCANLINE + HBLANK_FLAG_CYCLE + 32), 1)
        .unwrap();

    assert_eq!(machine.inspect16(IO_START + 0x54).unwrap(), 1);
    assert_eq!(machine.system.dma[0].source, EWRAM_START + 8);
    assert_eq!(machine.system.dma[0].destination, IO_START + 0x52);

    // Exactly 160 visible requests consume 160 source entries.
    machine.advance_to(Cycle(CYCLES_PER_FRAME - 1), 1).unwrap();
    assert_eq!(machine.system.dma[0].source, EWRAM_START + 160 * 4);
    let source_after_visible = machine.system.dma[0].source;

    // HBlank during VBlank must not advance the repeated source table.
    machine
        .advance_to(Cycle(CYCLES_PER_FRAME + HBLANK_FLAG_CYCLE - 1), 1)
        .unwrap();
    assert_eq!(machine.system.dma[0].source, source_after_visible);

    // Repeated transfers resume on the next visible line-zero HBlank.
    machine
        .advance_to(Cycle(CYCLES_PER_FRAME + HBLANK_FLAG_CYCLE + 32), 1)
        .unwrap();
    assert_eq!(machine.system.dma[0].source, source_after_visible + 4);
}

#[test]
fn simultaneous_hblank_dma_obeys_channel_priority() {
    let mut machine = Machine::new();
    let dma0_source = EWRAM_START + 0x0800;
    let dma3_source = EWRAM_START + 0x0900;
    let destination = EWRAM_START + 0x1000;
    machine.system.ewram[0x0800..0x0802].copy_from_slice(&0x1111u16.to_le_bytes());
    machine.system.ewram[0x0900..0x0902].copy_from_slice(&0x3333u16.to_le_bytes());

    let data = Access {
        kind: AccessKind::Data,
        sequential: false,
    };

    // Arm the lower-priority channel first to prove enable order is irrelevant.
    machine
        .system
        .write32_impl(IO_START + 0xd4, dma3_source, data)
        .unwrap();
    machine
        .system
        .write32_impl(IO_START + 0xd8, destination, data)
        .unwrap();
    machine
        .system
        .write32_impl(IO_START + 0xdc, (u32::from(0xa240u16) << 16) | 1, data)
        .unwrap();

    machine
        .system
        .write32_impl(IO_START + 0xb0, dma0_source, data)
        .unwrap();
    machine
        .system
        .write32_impl(IO_START + 0xb4, destination, data)
        .unwrap();
    machine
        .system
        .write32_impl(IO_START + 0xb8, (u32::from(0xa240u16) << 16) | 1, data)
        .unwrap();

    let edge = machine.system.next_visible_hblank();
    machine.system.advance_time(edge - machine.system.cycles);
    assert!(machine.system.dma[0].active);
    assert!(machine.system.dma[3].active);

    let dma3_before = machine.system.dma[3].source;
    machine.step().unwrap();
    assert_eq!(machine.inspect16(destination).unwrap(), 0x1111);
    assert!(!machine.system.dma[0].active);
    assert!(machine.system.dma[3].active);
    assert_eq!(machine.system.dma[3].source, dma3_before);

    machine.step().unwrap();
    assert_eq!(machine.inspect16(destination).unwrap(), 0x3333);
    assert!(!machine.system.dma[3].active);
    assert_eq!(machine.executed_instructions(), 0);
}

#[test]
fn dma0_latches_14_bit_count_and_27_bit_addresses() {
    let mut system = System::new();
    let data = Access {
        kind: AccessKind::Data,
        sequential: false,
    };

    system
        .write32_impl(IO_START + 0xb0, 0x0fff_fffd, data)
        .unwrap();
    system
        .write32_impl(IO_START + 0xb4, 0x0fff_fffd, data)
        .unwrap();
    // DMA0's 14-bit CNT_L turns 0x4001 into one transfer.
    system
        .write32_impl(IO_START + 0xb8, (u32::from(0xa000u16) << 16) | 0x4001, data)
        .unwrap();

    assert_eq!(system.dma[0].source, 0x07ff_fffc);
    assert_eq!(system.dma[0].destination, 0x07ff_fffc);
    assert_eq!(system.dma[0].count, 1);
}

#[test]
fn controlled_startup_is_arm_system_with_interrupts_masked() {
    let machine = Machine::new();

    assert_eq!(machine.cpu.registers[13], 0x0300_7F00);

    assert_eq!(machine.cpu.cpsr & 0xFF, CONTROLLED_START_CPSR);

    assert_eq!(machine.cpu.cpsr & CPSR_T, 0);
    assert_ne!(machine.cpu.cpsr & CPSR_I, 0);
    assert_ne!(machine.cpu.cpsr & CPSR_F, 0);
    assert_eq!(machine.cpu.cpsr & 0x1F, CPSR_SYSTEM_MODE);
}

#[test]
fn unchanged_button_state_does_not_retrigger_keypad_irq() {
    let mut machine = Machine::new();
    let access = Access {
        kind: AccessKind::Data,
        sequential: false,
    };

    // Select A, OR mode, IRQ enabled.
    machine
        .system
        .write16_impl(IO_START + 0x132, 0x4001, access)
        .unwrap();

    machine.set_button(Button::A, true);
    assert_eq!(
        machine.inspect16(IO_START + 0x202).unwrap() & 0x1000,
        0x1000
    );

    // Acknowledge keypad IF.
    machine
        .system
        .write16_impl(IO_START + 0x202, 0x1000, access)
        .unwrap();

    assert_eq!(machine.inspect16(IO_START + 0x202).unwrap() & 0x1000, 0);

    // Host polling the same state repeatedly must have no hardware effect.
    for _ in 0..100 {
        machine.set_button(Button::A, true);
    }

    assert_eq!(machine.inspect16(IO_START + 0x202).unwrap() & 0x1000, 0);

    // A genuine transition may request it again.
    machine.set_button(Button::A, false);
    machine.set_button(Button::A, true);

    assert_eq!(
        machine.inspect16(IO_START + 0x202).unwrap() & 0x1000,
        0x1000
    );
}

#[test]
fn firered_agbprint_init_rom_writes_do_not_fault_or_mutate_rom() {
    let mut bus = System::new();
    bus.rom = vec![0x5a; 16 * 1024 * 1024];

    let access = Access {
        kind: AccessKind::Data,
        sequential: false,
    };

    let original_prefix = bus.rom[..16].to_vec();

    // Retail FireRed Rev 0 retained AGBPrintInit. These are cartridge-ROM
    // writes used by that SDK debug facility. On an ordinary cartridge they
    // must not raise a CPU-visible memory fault.
    for (address, value) in [
        (0x09fe_2ffe, 0x0020),
        (0x09fe_20f8, 0x0000),
        (0x09fe_20fc, 0x0000),
        (0x09fe_20fe, 0x0000),
        (0x09fe_20fa, 0x00fd),
        (0x09fe_2ffe, 0x0000),
    ] {
        bus.write16_impl(address, value, access).unwrap();
    }

    assert_eq!(&bus.rom[..16], original_prefix.as_slice());

    // Ordinary in-range ROM is equally read-only.
    bus.write16_impl(ROM_START, 0x1234, access).unwrap();

    assert_eq!(&bus.rom[..16], original_prefix.as_slice());
}

#[test]
fn blend_targets_use_the_immediate_visible_second_surface() {
    let mut io = [0u8; IO_BYTES];

    // BG0 first target, BG1 second target, alpha mode.
    io[0x50..0x52].copy_from_slice(&0x0241u16.to_le_bytes());

    // Values above 16 clamp to 16 when used.
    io[0x52..0x54].copy_from_slice(&0x1f1fu16.to_le_bytes());

    let effects = ColorEffects::from_io(&io);

    assert_eq!(effects.eva, 16);
    assert_eq!(effects.evb, 16);

    let red = LayerPixel {
        color: 0x001f,
        layer: 0,
        priority: 0,
        semitransparent: false,
    };

    let blue = LayerPixel {
        color: 0x7c00,
        layer: 1,
        priority: 1,
        semitransparent: false,
    };

    let green = LayerPixel {
        color: 0x03e0,
        layer: 2,
        priority: 0,
        semitransparent: false,
    };

    let mut direct = PixelStack::new(0);
    direct.insert(blue);
    direct.insert(red);

    // 16/16 + 16/16 saturates red and blue.
    assert_eq!(effects.resolve(direct, true), 0x7c1f);

    let mut blocked = PixelStack::new(0);
    blocked.insert(blue);
    blocked.insert(green);
    blocked.insert(red);

    /*
     * BG2 is immediately below BG0 but is not selected as target 2.
     * Hardware must not skip BG2 and blend against deeper BG1.
     */
    assert_eq!(effects.resolve(blocked, true), 0x001f);
}

#[test]
fn semitransparent_obj_forces_alpha_before_window_effect_gate() {
    let effects = ColorEffects {
        // Ordinary fallback selects OBJ for darkening.
        first: 1 << BLEND_LAYER_OBJ,

        // Only BG0 is eligible beneath the semitransparent OBJ.
        second: 1 << 0,

        mode: 3,
        eva: 8,
        evb: 8,
        evy: 8,
    };

    let mut stack = PixelStack::new(0);

    stack.insert(LayerPixel {
        color: 0x001f,
        layer: 0,
        priority: 0,
        semitransparent: false,
    });

    stack.insert(LayerPixel {
        color: 0x7fff,
        layer: BLEND_LAYER_OBJ,
        priority: 0,
        semitransparent: true,
    });

    /*
     * Forced semitransparent alpha wins over BLDCNT darken and occurs even
     * when the ordinary window special-effects bit is clear.
     */
    assert_eq!(effects.resolve(stack, false), 0x41ff);

    let no_second_target = ColorEffects {
        second: 0,
        ..effects
    };

    // No eligible second surface and SFX disabled: unchanged.
    assert_eq!(no_second_target.resolve(stack, false), 0x7fff);

    // Without a forced blend source, normal darken behavior is allowed.
    assert_eq!(no_second_target.resolve(stack, true), 0x41f0);
}

#[test]
fn color_effect_arithmetic_matches_hardware_rounding_and_green_precision() {
    // Nearest-rounded 50/50 red + blue.
    assert_eq!(ColorEffects::alpha_blend(0x001f, 0x7c00, 8, 8), 0x4010);

    // RGB555 bit 15 contributes the hidden sixth green precision bit.
    assert_eq!(ColorEffects::alpha_blend(0x8000, 0x03e0, 8, 8), 0x0200);

    assert_eq!(ColorEffects::brighten(0x001f, 8), 0x421f);

    assert_eq!(ColorEffects::darken(0x001f, 8), 0x0010);
}

#[test]
fn unmapped_low_system_stores_are_ignored() {
    let mut machine = Machine::new();

    let data = Access {
        kind: AccessKind::Data,
        sequential: false,
    };

    machine.system.cpu_open_bus = 0x4433_2211;

    // BIOS ends at 0x00003fff. The remaining low system area has no
    // writable backing device, but stores must not terminate execution.
    machine.system.write8(0x0000_7c00, 0xaa, data).unwrap();

    machine
        .system
        .write16_impl(0x0000_7c00, 0xbbcc, data)
        .unwrap();

    machine
        .system
        .write32_impl(0x0000_7c00, 0xddee_ff00, data)
        .unwrap();

    // Reads remain open-bus rather than becoming writable storage.
    assert_eq!(machine.system.read8(0x0000_7c00, data).unwrap(), 0x11,);

    assert_eq!(
        machine.system.read16_impl(0x0000_7c00, data).unwrap(),
        0x2211,
    );

    assert_eq!(
        machine.system.read32_impl(0x0000_7c00, data).unwrap(),
        0x4433_2211,
    );
}

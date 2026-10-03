//! Machine ownership, boot/reset lifecycle, host input, and bounded execution.
//! CPU execution, bus scheduling, and display rendering are private sibling modules.

mod cpu;
mod display;
mod system;

#[cfg(feature = "eeprom-trace")]
use crate::EepromTrace;
use crate::{
    BIOS_SIZE, BackupSelection, BackupType, Button, CoreError, Cycle, EEPROM8K_BYTES,
    EEPROM512_BYTES, IO_BYTES, IO_START, MAX_ROM_BYTES, RtcImage, RunError, RunReport, SaveImage,
    SaveStatus, detect_backup, eeprom, flash, rtc, sram,
};
use cpu::{CPSR_F, CPSR_I, CPSR_T, Cpu, CpuBus};
use system::{System, range_for};

/// Ordered external input delivered on the same cycle timeline as hardware accesses.
pub(super) struct InputEvent {
    pub(super) cycle: Cycle,
    pub(super) button: Button,
    pub(super) pressed: bool,
}

/// Owns the CPU, memory, display, and cycle state for one emulated machine.
pub struct Machine {
    /// Reset retains the selected boot route; diagnostics remain controlled.
    bios_startup: bool,
    backup: BackupSelection,
    cpu: Cpu,
    system: System,
    executed_instructions: usize,
}

impl Machine {
    /// Applies a live host transition at the current instruction boundary.
    pub fn set_button(&mut self, button: Button, pressed: bool) {
        if !self.system.buttons.set(button, pressed) {
            return;
        }

        self.system.evaluate_keypad();
        self.system.refresh_status();
    }

    /// Reports the logical state actually visible to the guest keypad.
    pub fn button_pressed(&self, button: Button) -> bool {
        self.system.buttons.pressed(button)
    }
    /// Schedules a button transition on an absolute cycle. Events must be ordered;
    /// equal timestamps retain submission order, and the queue has a fixed bound.
    pub fn set_button_at(
        &mut self,
        cycle: Cycle,
        button: Button,
        pressed: bool,
    ) -> Result<(), CoreError> {
        let earliest = self
            .system
            .inputs
            .back()
            .map_or(self.cycles(), |event| event.cycle);
        if cycle < earliest {
            return Err(CoreError::InvalidInputTimestamp {
                requested: cycle,
                earliest,
            });
        }
        if self.system.inputs.len() >= 1024 {
            return Err(CoreError::InputQueueFull);
        }
        self.system.inputs.push_back(InputEvent {
            cycle,
            button,
            pressed,
        });
        self.system.next_device_event = self.system.cycles;
        self.system.advance_time(0);
        Ok(())
    }

    /// Cancels queued transitions and releases hardware keys at the current cycle.
    /// Used by pause/focus/reset boundaries so an old press cannot reappear later.
    pub fn release_all_buttons(&mut self) {
        self.system.inputs.clear();
        self.system.buttons.release_all();
        self.system.refresh_status();
    }
    /// Creates an unloaded machine at the GBA cartridge entry address.
    pub fn new() -> Self {
        Self::default()
    }

    /// Validates firmware before changing ownership. Loading does not execute or
    /// reset controlled diagnostics. An active BIOS session reboots with backup
    /// storage retained, preventing prefetched instructions from the previous image.
    pub fn load_bios(&mut self, bytes: &[u8]) -> Result<(), CoreError> {
        if bytes.len() != BIOS_SIZE {
            return Err(CoreError::InvalidBiosSize { size: bytes.len() });
        }
        let mut bios = Box::new([0u8; BIOS_SIZE]);
        bios.copy_from_slice(bytes);

        self.system.bios = Some(bios);
        if self.bios_startup {
            self.reset();
        }
        Ok(())
    }

    /// Boots a cartridge from the hardware reset vector. No diagnostic firmware
    /// or controlled stack/status state participates in this route.
    pub fn boot_rom_with_bios(
        &mut self,
        rom: &[u8],
        manual_override: Option<BackupType>,
    ) -> Result<(), CoreError> {
        if self.system.bios.is_none() {
            return Err(CoreError::MissingBios);
        }
        self.load_rom_with_backup(rom, manual_override)?;
        self.bios_startup = true;
        self.start_bios();
        Ok(())
    }

    /// Restores reset-vector execution in ARM supervisor mode with IRQ/FIQ
    /// masked. Firmware owns stack initialization and every subsequent service.
    fn start_bios(&mut self) {
        self.system.io[0x20..0x40].fill(0);
        self.cpu = Cpu::new();
        self.cpu.registers = [0; 16];
        self.cpu.cpsr = CPSR_I | CPSR_F | 0x13;
        self.system.execution_address = 0;
        self.system.bios_latch = 0;
        self.system.bios_enabled = true;
        self.system.test_firmware = false;
    }

    /// Loads a byte-based cartridge image and resets execution to its entry point.
    pub fn load_rom(&mut self, rom: &[u8]) -> Result<(), CoreError> {
        self.load_rom_with_backup(rom, None)
    }

    /// Loads a cartridge with a typed, validated override; failed loads retain state.
    pub fn load_rom_with_backup(
        &mut self,
        rom: &[u8],
        manual_override: Option<BackupType>,
    ) -> Result<(), CoreError> {
        if rom.is_empty() {
            return Err(CoreError::EmptyRom);
        }
        if rom.len() > MAX_ROM_BYTES {
            return Err(CoreError::RomTooLarge {
                size: rom.len(),
                maximum: MAX_ROM_BYTES,
            });
        }

        let bios = self.system.bios.take();
        *self = Self::new();
        self.system.bios = bios;
        self.backup = BackupSelection {
            detection: detect_backup(rom),
            manual_override,
        };
        if self.backup.selected() == Some(BackupType::Sram) {
            self.system.sram = Some(sram::Sram::new());
        } else if matches!(
            self.backup.selected(),
            Some(BackupType::Flash64 | BackupType::Flash128)
        ) {
            self.system.flash = Some(flash::Flash::new(
                self.backup.selected() == Some(BackupType::Flash128),
            ));
        }
        if matches!(
            self.backup.selected(),
            Some(BackupType::Eeprom | BackupType::Eeprom512 | BackupType::Eeprom8k)
        ) {
            let capacity = match self.backup.selected() {
                Some(BackupType::Eeprom512) => Some(EEPROM512_BYTES),
                Some(BackupType::Eeprom8k) => Some(EEPROM8K_BYTES),
                _ => None,
            };
            self.system.eeprom = Some(eeprom::Eeprom::new(capacity));
        }
        if rom.windows(9).any(|bytes| bytes == b"SIIRTC_V0") {
            self.system.rtc = Some(rtc::Rtc::new());
        }
        self.system.rom.extend_from_slice(rom);
        Ok(())
    }

    /// Samples externally supplied Unix seconds without advancing CPU work.
    pub fn set_rtc_time(&mut self, seconds: u64) {
        if let Some(chip) = &mut self.system.rtc {
            chip.set_time(seconds);
        }
    }

    /// Captures clock configuration independently of cartridge backup bytes.
    pub fn rtc_image(&self) -> Option<RtcImage> {
        self.system.rtc.as_ref().map(|chip| chip.image)
    }

    /// Validate and restore clock metadata before the first guest instruction.
    pub fn load_rtc(&mut self, bytes: &[u8]) -> Result<(), &'static str> {
        let image = RtcImage::decode(bytes)?;
        let chip = self.system.rtc.as_mut().ok_or("Cartridge has no RTC")?;
        chip.image = image;
        chip.initialized = true;
        chip.reset_bus();
        Ok(())
    }

    /// Acknowledge only the exact durable snapshot, retaining newer settings.
    pub fn acknowledge_rtc(&mut self, revision: u64) {
        if let Some(chip) = &mut self.system.rtc
            && chip.image.revision == revision
        {
            chip.image.dirty = false;
        }
    }

    /// Reports cartridge evidence and the effective save-hardware selection.
    pub fn backup_selection(&self) -> &BackupSelection {
        &self.backup
    }

    /// Start diagnostic capture after cartridge loading; production builds omit it.
    #[cfg(feature = "eeprom-trace")]
    pub fn enable_eeprom_trace(&mut self) {
        if let Some(eeprom) = &mut self.system.eeprom {
            eeprom.enable_trace();
        }
    }

    /// Drain transaction records without reading or clocking the cartridge bus.
    #[cfg(feature = "eeprom-trace")]
    pub fn drain_eeprom_trace(&mut self) -> (Vec<EepromTrace>, u64) {
        self.system
            .eeprom
            .as_mut()
            .map_or_else(|| (Vec::new(), 0), eeprom::Eeprom::drain_trace)
    }

    /// Copies cartridge bytes and their revision for asynchronous host storage.
    pub fn save_image(&self) -> Option<SaveImage> {
        self.system
            .sram
            .as_ref()
            .map(sram::Sram::image)
            .or_else(|| self.system.flash.as_ref().map(flash::Flash::image))
            .or_else(|| self.system.eeprom.as_ref().map(eeprom::Eeprom::image))
    }

    /// Reports cartridge save metadata without copying the owned backup bytes.
    pub fn save_status(&self) -> Option<SaveStatus> {
        self.system
            .sram
            .as_ref()
            .map(sram::Sram::status)
            .or_else(|| self.system.flash.as_ref().map(flash::Flash::status))
            .or_else(|| self.system.eeprom.as_ref().map(eeprom::Eeprom::status))
    }

    /// Loads validated initial bytes before execution, without making them dirty.
    pub fn load_save(&mut self, bytes: &[u8]) -> Result<(), &'static str> {
        if let Some(sram) = &mut self.system.sram {
            sram.load(bytes)
        } else if let Some(flash) = &mut self.system.flash {
            flash.load(bytes)
        } else if let Some(eeprom) = &mut self.system.eeprom {
            eeprom.load(bytes)
        } else {
            Err("cartridge has no supported backup hardware")
        }
    }

    /// Imports bytes as a newer revision, even when the content is unchanged.
    pub fn import_save(&mut self, bytes: &[u8]) -> Result<(), &'static str> {
        if let Some(sram) = &mut self.system.sram {
            sram.import(bytes)
        } else if let Some(flash) = &mut self.system.flash {
            flash.import(bytes)
        } else if let Some(eeprom) = &mut self.system.eeprom {
            eeprom.import(bytes)
        } else {
            Err("cartridge has no supported backup hardware")
        }
    }

    /// A completion can acknowledge only the revision actually written.
    pub fn acknowledge_save(&mut self, revision: u64) {
        if let Some(sram) = &mut self.system.sram {
            sram.acknowledge(revision);
        } else if let Some(flash) = &mut self.system.flash {
            flash.acknowledge(revision);
        } else if let Some(eeprom) = &mut self.system.eeprom {
            eeprom.acknowledge(revision);
        }
    }

    /// Enables the original SWI-division and IRQ-vector firmware for a controlled diagnostic
    /// session. Normal cartridge loading leaves the low firmware region unmapped.
    pub fn enable_test_firmware(&mut self) {
        if self.bios_startup {
            return;
        }
        self.system.test_firmware = true;
        self.cpu.exception_banks[1][0] = 0x0300_7fe0;
    }

    /// Drains completed mono PCM samples without advancing guest time.
    pub fn drain_pcm(&mut self, output: &mut Vec<f32>) {
        self.system.audio.drain(output);
    }

    /// Drains completed left/right PCM frames without advancing guest time.
    pub fn drain_stereo_pcm(&mut self, output: &mut Vec<[f32; 2]>) {
        self.system.audio.drain_stereo(output);
    }

    /// Discards host-bound samples at pause/focus boundaries, retaining devices.
    pub fn clear_pcm(&mut self) {
        self.system.audio.clear_pcm();
    }

    /// Reports produced, discarded and empty-FIFO sample counts since reset.
    pub fn pcm_counters(&self) -> (u64, u64, u64) {
        (
            self.system.audio.produced,
            self.system.audio.dropped,
            self.system.audio.fifo_underruns,
        )
    }

    /// Executes one instruction, one DMA read/write beat, or advances HALT to
    /// the next input/display event. DMA and sleep preserve instruction count.
    pub fn step(&mut self) -> Result<u32, CoreError> {
        if let Some(channel) = self.system.dma.iter().position(|dma| dma.active) {
            self.system.dma_beat(channel)?;
            return Ok(self.cpu.registers[15]);
        }
        let pending = self.system.pending_interrupts();
        if pending != 0 {
            self.system.halted = false;
            if self.system.io[0x208] & 1 != 0 && self.cpu.cpsr & CPSR_I == 0 {
                let status = self.cpu.cpsr;
                let resume = self.cpu.registers[15];
                self.cpu
                    .set_status((status & !(31 | CPSR_T)) | 0x12 | CPSR_I);
                self.cpu.saved_status[1] = status;
                self.cpu.registers[14] = resume.wrapping_add(4);
                // Delivery always fetches the mapped vector; no host callback shortcut.
                self.system.idle(1);
                self.cpu.branch(&mut self.system, 0x18, false)?;
            }
        }
        if self.system.halted {
            // Bound a public step to the next input/display event. advance_to additionally
            // clips sleeping time to its caller's absolute deadline.
            let next = self.system.next_wake_event();
            self.system.advance_time(next - self.system.cycles);
            return Ok(self.cpu.registers[15]);
        }
        let outcome = self.cpu.step(&mut self.system)?;
        self.executed_instructions = self.executed_instructions.saturating_add(1);
        Ok(outcome.address)
    }

    /// Advances toward an absolute cycle deadline with a hard instruction budget.
    /// An instruction or DMA beat may finish beyond the deadline; subsequent
    /// targets retain that excess. A complete DMA descriptor never runs atomically.
    pub fn advance_to(
        &mut self,
        target: Cycle,
        instruction_limit: usize,
    ) -> Result<RunReport, RunError> {
        let initial = self.executed_instructions;
        let mut last_pc = self.cpu.registers[15];
        while self.cycles() < target {
            if self.executed_instructions - initial >= instruction_limit {
                return Err(RunError::StepLimitExceeded {
                    limit: instruction_limit,
                    last_pc,
                    cycles: self.cycles(),
                });
            }
            // Between scheduled edges, ordinary instructions cannot introduce
            // DMA/IRQ/HALT work without MMIO. MMIO sets cpu_boundary; an access
            // crossing an event finishes its instruction, then exits this loop
            // before the next retirement. DMA keeps the public beat-at-a-time path.
            if !self.system.halted
                && self.system.pending_interrupts() == 0
                && !self.system.dma.iter().any(|dma| dma.active)
                && self.system.next_device_event > self.system.cycles
            {
                self.system.cpu_boundary = false;
                let deadline = target.0.min(self.system.next_device_event);
                let remaining = instruction_limit - (self.executed_instructions - initial);
                for _ in 0..remaining {
                    if self.system.cycles >= deadline || self.system.cpu_boundary {
                        break;
                    }
                    last_pc = self.cpu.step(&mut self.system)?.address;
                    self.executed_instructions = self.executed_instructions.saturating_add(1);
                }
                continue;
            }
            if self.system.halted
                && self.system.pending_interrupts() == 0
                && !self.system.dma.iter().any(|dma| dma.active)
            {
                let next = self.system.next_wake_event();
                let deadline = next.min(target.0);
                self.system.advance_time(deadline - self.system.cycles);
                continue;
            }
            last_pc = self.step()?;
        }
        Ok(RunReport {
            instructions: self.executed_instructions - initial,
            instruction_address: last_pc,
            cycles: self.cycles(),
        })
    }

    /// Executes a declared terminal instruction; arbitrary guest loops are not completion.
    pub fn run_until_pc(
        &mut self,
        terminal_pc: u32,
        instruction_limit: usize,
        cycle_limit: Cycle,
    ) -> Result<RunReport, RunError> {
        let initial = self.executed_instructions;
        for _ in 0..instruction_limit {
            let before = self.executed_instructions;
            let pc = self.step()?;
            if self.cycles() > cycle_limit {
                return Err(RunError::CycleLimitExceeded {
                    limit: cycle_limit,
                    reached: self.cycles(),
                    last_pc: pc,
                });
            }
            if pc == terminal_pc && self.executed_instructions != before {
                return Ok(RunReport {
                    instructions: self.executed_instructions - initial,
                    instruction_address: pc,
                    cycles: self.cycles(),
                });
            }
        }
        Err(RunError::StepLimitExceeded {
            limit: instruction_limit,
            last_pc: self.cpu.registers[15],
            cycles: self.cycles(),
        })
    }

    /// Reads diagnostic RAM without changing bus timing or device state.
    pub fn inspect16(&self, address: u32) -> Result<u16, CoreError> {
        if address & 1 != 0 {
            return Err(CoreError::InvalidAccessAlignment { address, width: 2 });
        }

        if let Some(range) = range_for(address, IO_START, IO_BYTES, 2) {
            let mut io = self.system.io.clone();
            self.system.refresh_readback(&mut io);
            return Ok(u16::from_le_bytes([io[range.start], io[range.start + 1]]));
        }
        let bytes = self.system.read_bytes(address, 2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    /// Reports hardware sleep for bounded fixture stall checks and application status.
    pub fn halted(&self) -> bool {
        self.system.halted
    }

    /// Generation of the last completed frame, independent of UI redraws.
    pub fn framebuffer_generation(&self) -> u64 {
        self.system.display.generation
    }

    /// Returns the last completed 16-bit framebuffer in row-major order.
    pub fn framebuffer(&self) -> &[u16] {
        &self.system.display.completed
    }

    /// Returns the current absolute hardware cycle position.
    pub fn cycles(&self) -> Cycle {
        Cycle(self.system.cycles)
    }

    /// Exposes architectural registers for manifest-bound diagnostic results.
    pub fn registers(&self) -> &[u32; 16] {
        &self.cpu.registers
    }

    /// Exposes status for diagnostic completion and exception-return checks.
    pub fn cpsr(&self) -> u32 {
        self.cpu.cpsr
    }

    /// Reports the active instruction set for bounded guest-return diagnostics.
    pub fn is_thumb(&self) -> bool {
        self.cpu.cpsr & CPSR_T != 0
    }

    /// Returns the number of successfully executed guest instructions since reset.
    pub fn executed_instructions(&self) -> usize {
        self.executed_instructions
    }

    /// Resets CPU, memory, and display state while retaining the loaded ROM bytes.
    pub fn reset(&mut self) {
        let rom = std::mem::take(&mut self.system.rom);
        let firmware = self.system.test_firmware;
        let bios = self.system.bios.take();
        let bios_startup = self.bios_startup;
        let mut rtc = self.system.rtc.take();
        if let Some(chip) = &mut rtc {
            chip.reset_bus();
        }
        let backup = std::mem::take(&mut self.backup);
        let sram = self.system.sram.take();
        let mut eeprom = self.system.eeprom.take();
        if let Some(chip) = &mut eeprom {
            chip.reset();
        }
        let mut flash = self.system.flash.take();
        if let Some(chip) = &mut flash {
            chip.reset();
        }
        *self = Self::new();
        self.system.rom = rom;
        self.system.rtc = rtc;
        self.system.bios = bios;
        self.bios_startup = bios_startup;
        if bios_startup {
            self.start_bios();
        }
        self.backup = backup;
        self.system.sram = sram;
        self.system.flash = flash;
        self.system.eeprom = eeprom;
        if firmware {
            self.enable_test_firmware();
        }
    }
}

impl Default for Machine {
    fn default() -> Self {
        Self {
            bios_startup: false,
            backup: BackupSelection::default(),
            cpu: Cpu::new(),
            system: System::new(),
            executed_instructions: 0,
        }
    }
}

#[cfg(test)]
mod tests;

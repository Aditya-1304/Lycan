//! ARM7TDMI registers, pipeline, instruction execution, and the CPU bus contract.

use crate::{CoreError, ROM_START};

pub(super) const CPSR_N: u32 = 1 << 31;
pub(super) const CPSR_Z: u32 = 1 << 30;
pub(super) const CPSR_C: u32 = 1 << 29;
pub(super) const CPSR_V: u32 = 1 << 28;

pub(super) const CPSR_I: u32 = 1 << 7;
pub(super) const CPSR_F: u32 = 1 << 6;
pub(super) const CPSR_T: u32 = 1 << 5;

pub(super) const CPSR_SYSTEM_MODE: u32 = 0x1F;

pub(super) const CONTROLLED_START_CPSR: u32 = CPSR_I | CPSR_F | CPSR_SYSTEM_MODE;

pub(super) const CONTROLLED_START_SP: u32 = 0x0300_7F00;

/// Access classification remains explicit at the CPU/bus boundary.
#[derive(Clone, Copy)]
pub(super) enum AccessKind {
    Fetch,
    Data,
}

#[derive(Clone, Copy)]
pub(super) struct Access {
    pub(super) kind: AccessKind,
    pub(super) sequential: bool,
}

pub(super) trait CpuBus {
    /// Supplies the executing instruction address independently of pipeline lookahead.
    fn set_execution_address(&mut self, address: u32);
    /// Publishes instruction bus context before any operand accesses overwrite data latches.
    fn set_cpu_open_bus(&mut self, pipeline: [u32; 2], address: u32, thumb: bool);
    fn read8(&mut self, address: u32, access: Access) -> Result<u8, CoreError>;
    fn write8(&mut self, address: u32, value: u8, access: Access) -> Result<(), CoreError>;
    fn read16(&mut self, address: u32, access: Access) -> Result<u16, CoreError>;
    fn read32(&mut self, address: u32, access: Access) -> Result<u32, CoreError>;

    /// Opcode transfers retain fetch timing while bypassing data-only devices.
    fn fetch16(&mut self, address: u32, access: Access) -> Result<u16, CoreError>;
    fn fetch32(&mut self, address: u32, access: Access) -> Result<u32, CoreError>;

    fn write16(&mut self, address: u32, value: u16, access: Access) -> Result<(), CoreError>;
    fn write32(&mut self, address: u32, value: u32, access: Access) -> Result<(), CoreError>;

    fn idle(&mut self, cycles: u64);
    /// Signals a pipeline redirect; the bus discards cartridge prefetch state.
    fn restart_fetch(&mut self);
}

pub(super) struct Cpu {
    pub(super) registers: [u32; 16],
    pipeline: [u32; 2],
    pipeline_valid: bool,
    next_fetch_is_sequential: bool,
    pub(super) cpsr: u32,
    /// User/system and FIQ banks hold r8-r14 across mode changes.
    user_bank: [u32; 7],
    fiq_bank: [u32; 7],
    /// IRQ, supervisor, abort, and undefined modes bank SP/LR and SPSR.
    pub(super) exception_banks: [[u32; 2]; 4],
    pub(super) saved_status: [u32; 5],
}

impl Cpu {
    pub(super) fn new() -> Self {
        let mut registers = [0; 16];

        registers[13] = CONTROLLED_START_SP;
        registers[15] = ROM_START;

        Self {
            registers,
            pipeline: [0; 2],
            pipeline_valid: false,
            next_fetch_is_sequential: true,
            cpsr: CONTROLLED_START_CPSR,
            user_bank: [0; 7],
            fiq_bank: [0; 7],
            exception_banks: [[0; 2]; 4],
            saved_status: [0; 5],
        }
    }

    /// Returns the privileged bank index; user and system share one bank.
    fn status_bank(mode: u32) -> Option<usize> {
        match mode & 31 {
            0x11 => Some(0),
            0x12 => Some(1),
            0x13 => Some(2),
            0x17 => Some(3),
            0x1b => Some(4),
            _ => None,
        }
    }

    /// Saves outgoing banked registers before exposing the incoming mode.
    pub(super) fn set_status(&mut self, value: u32) {
        let old = Self::status_bank(self.cpsr);
        let new = Self::status_bank(value);
        if old != new {
            if old == Some(0) {
                self.fiq_bank.copy_from_slice(&self.registers[8..15]);
                self.registers[8..15].copy_from_slice(&self.user_bank);
            } else {
                self.user_bank[..5].copy_from_slice(&self.registers[8..13]);
                if let Some(bank) = old {
                    self.exception_banks[bank - 1].copy_from_slice(&self.registers[13..15]);
                } else {
                    self.user_bank[5..].copy_from_slice(&self.registers[13..15]);
                }
            }
            if new == Some(0) {
                self.registers[8..15].copy_from_slice(&self.fiq_bank);
            } else if let Some(bank) = new {
                self.registers[13..15].copy_from_slice(&self.exception_banks[bank - 1]);
            } else {
                self.registers[13..15].copy_from_slice(&self.user_bank[5..]);
            }
        }
        self.cpsr = value;
    }

    /// MRS/MSR apply byte-field masks and privilege rules before ALU decoding.
    fn execute_status_transfer(&mut self, instruction: u32) -> bool {
        let saved = instruction & (1 << 22) != 0;
        if instruction & 0x0fbf_0fff == 0x010f_0000 {
            let rd = ((instruction >> 12) & 15) as usize;
            self.registers[rd] = if saved {
                Self::status_bank(self.cpsr).map_or(self.cpsr, |b| self.saved_status[b])
            } else {
                self.cpsr
            };
            return true;
        }
        if instruction & 0x0db0_f000 != 0x0120_f000 {
            return false;
        }
        let value = if instruction & (1 << 25) != 0 {
            (instruction & 255).rotate_right(((instruction >> 8) & 15) * 2)
        } else {
            self.registers[(instruction & 15) as usize]
        };
        let mut mask = 0;
        for byte in 0..4 {
            if instruction & (1 << (16 + byte)) != 0 {
                mask |= 255 << (byte * 8);
            }
        }
        if saved {
            if let Some(bank) = Self::status_bank(self.cpsr) {
                self.saved_status[bank] = (self.saved_status[bank] & !mask) | (value & mask);
            }
        } else {
            if self.cpsr & 31 == 0x10 {
                mask &= 0xff00_0000;
            }
            // ARMv4 MSR cannot switch instruction sets; exception return can.
            mask &= !CPSR_T;
            self.set_status((self.cpsr & !mask) | (value & mask));
        }
        true
    }

    /// SWI enters supervisor ARM state, banks LR/SP, and maps the return
    /// state through SPSR. Service execution remains ordinary guest code.
    fn software_interrupt<B: CpuBus>(
        &mut self,
        bus: &mut B,
        return_address: u32,
    ) -> Result<(), CoreError> {
        let status = self.cpsr;
        self.set_status((status & !(31 | CPSR_T)) | 0x13 | CPSR_I);
        self.saved_status[2] = status;
        self.registers[14] = return_address;
        self.branch(bus, 8, false)
    }

    /// Fills the two-stage instruction pipeline after reset or a taken branch.
    fn refill<B: CpuBus>(&mut self, bus: &mut B) -> Result<(), CoreError> {
        let pc = self.registers[15];
        let thumb = self.cpsr & CPSR_T != 0;
        let width = if thumb { 2 } else { 4 };
        bus.set_execution_address(pc);
        bus.restart_fetch();
        self.pipeline[0] = Self::fetch(
            bus,
            pc,
            thumb,
            Access {
                kind: AccessKind::Fetch,
                sequential: false,
            },
        )?;
        self.pipeline[1] = Self::fetch(
            bus,
            pc.wrapping_add(width),
            thumb,
            Access {
                kind: AccessKind::Fetch,
                sequential: true,
            },
        )?;
        self.pipeline_valid = true;
        self.next_fetch_is_sequential = true;
        Ok(())
    }

    /// Instruction width follows CPSR.T; register 15 stores the next executed
    /// address, while individual operands expose the architectural pipeline PC.
    fn instruction_width(&self) -> u32 {
        if self.cpsr & CPSR_T != 0 { 2 } else { 4 }
    }

    fn fetch<B: CpuBus>(
        bus: &mut B,
        address: u32,
        thumb: bool,
        access: Access,
    ) -> Result<u32, CoreError> {
        if thumb {
            bus.fetch16(address, access).map(u32::from)
        } else {
            bus.fetch32(address, access)
        }
    }

    /// BX alone selects state from bit zero. Ordinary PC writes retain state;
    /// every taken branch discards the old pipeline before fetching its target.
    pub(super) fn branch<B: CpuBus>(
        &mut self,
        bus: &mut B,
        target: u32,
        exchange: bool,
    ) -> Result<(), CoreError> {
        if exchange {
            self.cpsr = (self.cpsr & !CPSR_T) | if target & 1 != 0 { CPSR_T } else { 0 };
        }
        self.registers[15] = target & !(self.instruction_width() - 1);
        self.refill(bus)
    }

    /// Executes from the pipeline and accounts for fetch, data, and internal cycles once.
    pub(super) fn step<B: CpuBus>(&mut self, bus: &mut B) -> Result<StepOutcome, CoreError> {
        if !self.pipeline_valid {
            self.refill(bus)?;
        }
        let address = self.registers[15];
        bus.set_execution_address(address);
        let instruction = self.pipeline[0];
        // Execution state is stable until this instruction executes. Share it
        // across opcode width, fetch and open-bus projection before dispatch.
        let thumb = self.cpsr & CPSR_T != 0;
        let width = if thumb { 2 } else { 4 };
        let next = Self::fetch(
            bus,
            address.wrapping_add(width * 2),
            thumb,
            Access {
                kind: AccessKind::Fetch,
                sequential: self.next_fetch_is_sequential,
            },
        )?;
        self.pipeline = [self.pipeline[1], next];
        bus.set_cpu_open_bus(self.pipeline, address, thumb);
        self.registers[15] = address.wrapping_add(width);
        self.next_fetch_is_sequential = true;
        if thumb {
            self.execute_thumb(address, instruction as u16, bus)?;
            return Ok(StepOutcome { address });
        }
        if !self.condition_passes(instruction >> 28) {
            return Ok(StepOutcome { address });
        }
        // Bits 27..25 select disjoint ARM instruction families. Resolve that
        // family once; only the overlapping ALU/status/multiply encodings need
        // their detailed masks. Helpers retain validation of reserved forms.
        let executed = match (instruction >> 25) & 7 {
            5 => {
                if instruction & (1 << 24) != 0 {
                    self.registers[14] = address.wrapping_add(4);
                }
                let offset = ((instruction & 0x00ff_ffff) << 8) as i32 >> 6;
                self.registers[15] = address.wrapping_add(8).wrapping_add(offset as u32);
                self.refill(bus)?;
                true
            }
            7 if instruction & (1 << 24) != 0 => {
                self.software_interrupt(bus, address.wrapping_add(4))?;
                true
            }
            0 | 1 => {
                if instruction & 0x0fff_fff0 == 0x012f_ff10 {
                    let source = (instruction & 15) as usize;
                    let target = if source == 15 {
                        address.wrapping_add(8)
                    } else {
                        self.registers[source]
                    };
                    self.branch(bus, target, true)?;
                    true
                } else if self.execute_status_transfer(instruction)
                    || self.execute_multiply(instruction, bus)
                {
                    true
                } else if self.execute_data_processing(address, instruction) {
                    if instruction & (1 << 25) == 0 && instruction & (1 << 4) != 0 {
                        bus.idle(1);
                    }
                    if (instruction >> 12) & 15 == 15 && !matches!((instruction >> 21) & 15, 8..=11)
                    {
                        self.branch(bus, self.registers[15], false)?;
                    }
                    true
                } else if self.execute_swap(instruction, bus)?
                    || self.execute_halfword(address, instruction, bus)?
                {
                    self.next_fetch_is_sequential = false;
                    true
                } else {
                    false
                }
            }
            2 | 3 => {
                let executed = self.execute_word_transfer(address, instruction, bus)?;
                if executed {
                    self.next_fetch_is_sequential =
                        instruction & (1 << 20) != 0 && (instruction >> 12) & 15 == 15;
                }
                executed
            }
            4 => {
                let executed = self.execute_block_transfer(address, instruction, bus)?;
                if executed {
                    self.next_fetch_is_sequential =
                        instruction & (1 << 20) != 0 && instruction & 0x8000 != 0;
                }
                executed
            }
            _ => false,
        };
        if !executed {
            return Err(CoreError::UnsupportedInstruction {
                address,
                instruction,
            });
        }
        Ok(StepOutcome { address })
    }

    /// Decodes ARMv4T Thumb forms into the shared ALU, transfer, and pipeline
    /// paths. Unsupported encodings remain explicit execution errors.
    fn execute_thumb<B: CpuBus>(
        &mut self,
        address: u32,
        instruction: u16,
        bus: &mut B,
    ) -> Result<(), CoreError> {
        let op = u32::from(instruction);
        let rd = (op & 7) as usize;
        let rn = ((op >> 3) & 7) as usize;
        let access = Access {
            kind: AccessKind::Data,
            sequential: false,
        };
        if op & 0xe000 == 0 && op & 0x1800 != 0x1800 {
            // Immediate shifts share ARM's zero-shift encodings and carry rules.
            let arm = (13 << 21)
                | (1 << 20)
                | ((rd as u32) << 12)
                | (((op >> 11) & 3) << 5)
                | (((op >> 6) & 31) << 7)
                | rn as u32;
            self.execute_data_processing(address, arm);
        } else if op & 0xf800 == 0x1800 {
            let operand = (op >> 6) & 7;
            let arm = (if op & 0x0400 != 0 { 1 << 25 } else { 0 })
                | ((if op & 0x0200 != 0 { 2 } else { 4 }) << 21)
                | (1 << 20)
                | ((rn as u32) << 16)
                | ((rd as u32) << 12)
                | operand;
            self.execute_data_processing(address, arm);
        } else if op & 0xe000 == 0x2000 {
            let destination = (op >> 8) & 7;
            let opcode = [13, 10, 4, 2][((op >> 11) & 3) as usize];
            let arm = (1 << 25)
                | (opcode << 21)
                | (1 << 20)
                | (destination << 16)
                | (destination << 12)
                | (op & 255);
            self.execute_data_processing(address, arm);
        } else if op & 0xfc00 == 0x4000 {
            let operation = (op >> 6) & 15;
            if matches!(operation, 2..=4 | 7) {
                let kind = if operation == 7 { 3 } else { operation - 2 };
                let arm = (13 << 21)
                    | (1 << 20)
                    | ((rd as u32) << 12)
                    | ((rn as u32) << 8)
                    | (kind << 5)
                    | (1 << 4)
                    | rd as u32;
                self.execute_data_processing(address, arm);
                bus.idle(1);
            } else if operation == 13 {
                let arm = (1 << 20) | ((rd as u32) << 16) | ((rn as u32) << 8) | 0x90 | rd as u32;
                self.execute_multiply(arm, bus);
            } else {
                let opcode =
                    [0, 1, 0, 0, 0, 5, 6, 0, 8, 3, 10, 11, 12, 0, 14, 15][operation as usize];
                let arm = (opcode << 21)
                    | (1 << 20)
                    | rn as u32
                    | ((rd as u32) << 16)
                    | ((rd as u32) << 12);
                let arm = if operation == 9 {
                    (arm & !0x000f_000f) | (1 << 25) | ((rn as u32) << 16)
                } else {
                    arm
                };
                self.execute_data_processing(address, arm);
            }
        } else if op & 0xfc00 == 0x4400 {
            let destination = ((op & 7) | ((op >> 4) & 8)) as usize;
            let source = ((op >> 3) & 15) as usize;
            let operand = if source == 15 {
                address.wrapping_add(4)
            } else {
                self.registers[source]
            };
            match (op >> 8) & 3 {
                0 | 2 => {
                    let lhs = if destination == 15 {
                        address.wrapping_add(4)
                    } else {
                        self.registers[destination]
                    };
                    let value = if op & 0x0200 == 0 {
                        lhs.wrapping_add(operand)
                    } else {
                        operand
                    };
                    if destination == 15 {
                        self.branch(bus, value, false)?;
                    } else {
                        self.registers[destination] = value;
                    }
                }
                1 => {
                    let arm = (10 << 21) | (1 << 20) | ((destination as u32) << 16) | source as u32;
                    self.execute_data_processing(address.wrapping_sub(4), arm);
                }
                3 if op & 0x0087 == 0 => self.branch(bus, operand, true)?,
                _ => {
                    return Err(CoreError::UnsupportedInstruction {
                        address,
                        instruction: op,
                    });
                }
            }
        } else if op & 0xf800 == 0x4800 {
            let target = (address.wrapping_add(4) & !3).wrapping_add((op & 255) * 4);
            self.registers[((op >> 8) & 7) as usize] = bus.read32(target, access)?;
            bus.idle(1);
            self.next_fetch_is_sequential = false;
        } else if op & 0xf000 == 0x5000 {
            let target = self.registers[rn].wrapping_add(self.registers[((op >> 6) & 7) as usize]);
            match (op >> 9) & 7 {
                0 => bus.write32(store_address(target, 3), self.registers[rd], access)?,
                1 => bus.write16(store_address(target, 1), self.registers[rd] as u16, access)?,
                2 => bus.write8(target, self.registers[rd] as u8, access)?,
                3 => self.registers[rd] = self.load_halfword_signed(bus, target, 2, access)?,
                4 => {
                    self.registers[rd] = bus
                        .read32(target & !3, access)?
                        .rotate_right((target & 3) * 8)
                }
                5 => self.registers[rd] = self.load_halfword_signed(bus, target, 1, access)?,
                6 => self.registers[rd] = u32::from(bus.read8(target, access)?),
                7 => self.registers[rd] = self.load_halfword_signed(bus, target, 3, access)?,
                _ => {
                    return Err(CoreError::UnsupportedInstruction {
                        address,
                        instruction: op,
                    });
                }
            }
            if op & 0x0800 != 0 || op & 0x0e00 == 0x0600 {
                bus.idle(1);
            }
            self.next_fetch_is_sequential = false;
        } else if op & 0xe000 == 0x6000 || op & 0xf000 == 0x8000 {
            let halfword = op & 0xf000 == 0x8000;
            let byte = !halfword && op & 0x1000 != 0;
            let offset = ((op >> 6) & 31)
                * if halfword {
                    2
                } else if byte {
                    1
                } else {
                    4
                };
            let target = self.registers[rn].wrapping_add(offset);
            if op & 0x0800 != 0 {
                self.registers[rd] = if halfword {
                    self.load_halfword_signed(bus, target, 1, access)?
                } else if byte {
                    u32::from(bus.read8(target, access)?)
                } else {
                    bus.read32(target & !3, access)?
                        .rotate_right((target & 3) * 8)
                };
                bus.idle(1);
            } else if halfword {
                bus.write16(store_address(target, 1), self.registers[rd] as u16, access)?;
            } else if byte {
                bus.write8(target, self.registers[rd] as u8, access)?;
            } else {
                bus.write32(store_address(target, 3), self.registers[rd], access)?;
            }
            self.next_fetch_is_sequential = false;
        } else if op & 0xf000 == 0x9000 {
            let target = self.registers[13].wrapping_add((op & 255) * 4);
            let register = ((op >> 8) & 7) as usize;
            if op & 0x0800 != 0 {
                self.registers[register] = bus
                    .read32(target & !3, access)?
                    .rotate_right((target & 3) * 8);
                bus.idle(1);
            } else {
                bus.write32(store_address(target, 3), self.registers[register], access)?;
            }
            self.next_fetch_is_sequential = false;
        } else if op & 0xf000 == 0xa000 {
            let base = if op & 0x0800 == 0 {
                address.wrapping_add(4) & !3
            } else {
                self.registers[13]
            };
            self.registers[((op >> 8) & 7) as usize] = base.wrapping_add((op & 255) * 4);
        } else if op & 0xf600 == 0xb400 {
            let load = op & 0x0800 != 0;
            let list = (op & 255)
                | if op & 0x0100 != 0 {
                    1 << if load { 15 } else { 14 }
                } else {
                    0
                };
            let arm = 0x0800_0000
                | (13 << 16)
                | (1 << 21)
                | list
                | if load { (1 << 23) | (1 << 20) } else { 1 << 24 };
            self.execute_block_transfer(address, arm, bus)?;
            self.next_fetch_is_sequential = load && list & (1 << 15) != 0;
        } else if op & 0xf000 == 0xc000 {
            let list = op & 255;
            let load = op & 0x0800 != 0;
            let arm = 0x08a0_0000 | (((op >> 8) & 7) << 16) | list | if load { 1 << 20 } else { 0 };
            self.execute_block_transfer(address, arm, bus)?;
            self.next_fetch_is_sequential = load && list == 0;
        } else if op & 0xff00 == 0xb000 {
            let offset = (op & 127) * 4;
            self.registers[13] = if op & 128 == 0 {
                self.registers[13].wrapping_add(offset)
            } else {
                self.registers[13].wrapping_sub(offset)
            };
        } else if op & 0xff00 == 0xdf00 {
            self.software_interrupt(bus, address.wrapping_add(2))?;
        } else if op & 0xf000 == 0xd000 && (op >> 8) & 15 < 14 {
            if self.condition_passes((op >> 8) & 15) {
                let offset = (instruction as u8 as i8 as i32) * 2;
                self.branch(
                    bus,
                    address.wrapping_add(4).wrapping_add(offset as u32),
                    false,
                )?;
            }
        } else if op & 0xf800 == 0xe000 {
            let offset = ((op & 0x07ff) << 21) as i32 >> 20;
            self.branch(
                bus,
                address.wrapping_add(4).wrapping_add(offset as u32),
                false,
            )?;
        } else if op & 0xf800 == 0xf000 {
            // ARMv4T BL is two independently executed halfwords. The prefix
            // places the sign-extended high offset in LR; the suffix tags return.
            let offset = ((op & 0x07ff) << 21) as i32 >> 9;
            self.registers[14] = address.wrapping_add(4).wrapping_add(offset as u32);
        } else if op & 0xf800 == 0xf800 {
            let target = self.registers[14].wrapping_add((op & 0x07ff) * 2);
            self.registers[14] = address.wrapping_add(2) | 1;
            self.branch(bus, target, false)?;
        } else {
            return Err(CoreError::UnsupportedInstruction {
                address,
                instruction: op,
            });
        }
        Ok(())
    }

    fn condition_passes(&self, condition: u32) -> bool {
        let n = self.cpsr & CPSR_N != 0;
        let z = self.cpsr & CPSR_Z != 0;
        let c = self.cpsr & CPSR_C != 0;
        let v = self.cpsr & CPSR_V != 0;

        match condition {
            0 => z,
            1 => !z,
            2 => c,
            3 => !c,
            4 => n,
            5 => !n,
            6 => v,
            7 => !v,
            8 => c && !z,
            9 => !c || z,
            10 => n == v,
            11 => n != v,
            12 => !z && n == v,
            13 => z || n != v,
            14 => true,
            _ => false,
        }
    }

    /// Executes ARM ALU operations with immediate or register-controlled shifts.
    /// Arithmetic shares carry/overflow rules; logical operations preserve V.
    /// S-bit PC destinations restore SPSR, including legacy comparison forms;
    /// only operations writing PC redirect execution and refill the pipeline.
    fn execute_data_processing(&mut self, address: u32, instruction: u32) -> bool {
        if instruction & 0x0C00_0000 != 0
            || (instruction & (1 << 25) == 0 && instruction & 0x90 == 0x90)
        {
            return false;
        }
        let opcode = (instruction >> 21) & 15;
        let set_flags = instruction & (1 << 20) != 0;
        if matches!(opcode, 8..=11) && !set_flags {
            return false;
        }
        let destination = ((instruction >> 12) & 15) as usize;
        let source = ((instruction >> 16) & 15) as usize;
        let lhs = if source == 15 {
            address.wrapping_add(if instruction & 0x0200_0010 == 0x10 {
                12
            } else {
                8
            })
        } else {
            self.registers[source]
        };
        let (operand, shifter_carry) = if instruction & (1 << 25) != 0 {
            let rotate = ((instruction >> 8) & 15) * 2;
            let value = (instruction & 255).rotate_right(rotate);
            (
                value,
                if rotate == 0 {
                    self.cpsr & CPSR_C != 0
                } else {
                    value >> 31 != 0
                },
            )
        } else {
            let source = (instruction & 15) as usize;
            let value = if source == 15 {
                address.wrapping_add(8)
            } else {
                self.registers[source]
            };
            if instruction & (1 << 4) != 0 {
                let value = if source == 15 {
                    address.wrapping_add(12)
                } else {
                    value
                };
                let rs = ((instruction >> 8) & 15) as usize;
                let amount = if rs == 15 {
                    address.wrapping_add(8)
                } else {
                    self.registers[rs]
                } & 255;
                self.shift_register(value, (instruction >> 5) & 3, amount)
            } else {
                self.shift_immediate(value, (instruction >> 5) & 3, (instruction >> 7) & 31)
            }
        };
        let carry_in = u32::from(self.cpsr & CPSR_C != 0);
        let arithmetic = match opcode {
            2 | 10 => Some((lhs, !operand, 1)),
            3 => Some((operand, !lhs, 1)),
            4 | 11 => Some((lhs, operand, 0)),
            5 => Some((lhs, operand, carry_in)),
            6 => Some((lhs, !operand, carry_in)),
            7 => Some((operand, !lhs, carry_in)),
            _ => None,
        };
        let (result, carry, overflow) = if let Some((a, b, carry)) = arithmetic {
            // Complementing the subtrahend makes carry mean "no borrow" for
            // subtraction, including SBC/RSC's inverted carry-in borrow.
            let wide = u64::from(a) + u64::from(b) + u64::from(carry);
            let result = wide as u32;
            (
                result,
                wide > u64::from(u32::MAX),
                (!(a ^ b) & (a ^ result)) & 0x8000_0000 != 0,
            )
        } else {
            let result = match opcode {
                0 | 8 => lhs & operand,
                1 | 9 => lhs ^ operand,
                12 => lhs | operand,
                13 => operand,
                14 => lhs & !operand,
                15 => !operand,
                _ => return false,
            };
            (result, shifter_carry, false)
        };
        if set_flags && destination == 15 {
            if let Some(bank) = Self::status_bank(self.cpsr) {
                self.set_status(self.saved_status[bank]);
            }
        } else if set_flags {
            self.cpsr &= !(CPSR_N | CPSR_Z);
            if result & 0x8000_0000 != 0 {
                self.cpsr |= CPSR_N;
            }
            if result == 0 {
                self.cpsr |= CPSR_Z;
            }
            if arithmetic.is_some() {
                self.cpsr &= !(CPSR_C | CPSR_V);
                if carry {
                    self.cpsr |= CPSR_C;
                }
                if overflow {
                    self.cpsr |= CPSR_V;
                }
            } else {
                self.cpsr = (self.cpsr & !CPSR_C) | if shifter_carry { CPSR_C } else { 0 };
            }
        }
        if !matches!(opcode, 8..=11) {
            self.registers[destination] = result;
        }
        true
    }

    /// Register shifts use an eight-bit count, preserving carry at zero and
    /// handling counts at or beyond the word width without host overshifts.
    fn shift_register(&self, value: u32, kind: u32, amount: u32) -> (u32, bool) {
        if amount == 0 {
            return (value, self.cpsr & CPSR_C != 0);
        }
        match kind & 3 {
            0 if amount < 32 => (value << amount, value & (1 << (32 - amount)) != 0),
            0 => (0, amount == 32 && value & 1 != 0),
            1 if amount < 32 => (value >> amount, value & (1 << (amount - 1)) != 0),
            1 => (0, amount == 32 && value >> 31 != 0),
            2 => (
                ((value as i32) >> amount.min(31)) as u32,
                value & (1 << (amount.min(32) - 1)) != 0,
            ),
            _ => {
                let result = value.rotate_right(amount);
                (result, result >> 31 != 0)
            }
        }
    }

    /// Decodes ARM immediate shifts, including the zero encodings for shifts
    /// by 32 and RRX. The carry result is used only when the ALU updates flags.
    fn shift_immediate(&self, value: u32, kind: u32, amount: u32) -> (u32, bool) {
        let carry = self.cpsr & CPSR_C != 0;
        match (kind & 3, amount) {
            (0, 0) => (value, carry),
            (0, n) => (value << n, value & (1 << (32 - n)) != 0),
            (1, 0) => (0, value >> 31 != 0),
            (1, n) => (value >> n, value & (1 << (n - 1)) != 0),
            (2, 0) => (((value as i32) >> 31) as u32, value >> 31 != 0),
            (2, n) => (((value as i32) >> n) as u32, value & (1 << (n - 1)) != 0),
            (_, 0) => ((u32::from(carry) << 31) | (value >> 1), value & 1 != 0),
            (_, n) => (value.rotate_right(n), value & (1 << (n - 1)) != 0),
        }
    }

    /// Executes short and signed/unsigned long multiplies, including accumulate
    /// forms. Internal timing follows ARM7 multiplier early termination.
    fn execute_multiply<B: CpuBus>(&mut self, instruction: u32, bus: &mut B) -> bool {
        if instruction & 0x0f80_00f0 == 0x0080_0090 {
            let hi = ((instruction >> 16) & 15) as usize;
            let lo = ((instruction >> 12) & 15) as usize;
            let rs = ((instruction >> 8) & 15) as usize;
            let rm = (instruction & 15) as usize;
            let signed = instruction & (1 << 22) != 0;
            let accumulate = instruction & (1 << 21) != 0;
            let multiplier = self.registers[rs];
            let mut result = if signed {
                (i64::from(self.registers[rm] as i32) * i64::from(multiplier as i32)) as u64
            } else {
                u64::from(self.registers[rm]) * u64::from(multiplier)
            };
            if accumulate {
                result = result.wrapping_add(
                    (u64::from(self.registers[hi]) << 32) | u64::from(self.registers[lo]),
                );
            }
            self.registers[lo] = result as u32;
            self.registers[hi] = (result >> 32) as u32;
            if instruction & (1 << 20) != 0 {
                self.cpsr = (self.cpsr & !(CPSR_N | CPSR_Z))
                    | if result >> 63 != 0 { CPSR_N } else { 0 }
                    | if result == 0 { CPSR_Z } else { 0 };
            }
            let iterations = [8_u32, 16, 24]
                .into_iter()
                .find(|bits| {
                    let upper = multiplier >> bits;
                    upper == 0 || (signed && (multiplier as i32) >> bits == -1)
                })
                .map_or(4, |bits| bits / 8);
            bus.idle(u64::from(iterations) + 1 + u64::from(accumulate));
            return true;
        }
        if instruction & 0x0FC0_00F0 != 0x0000_0090 {
            return false;
        }
        let rd = ((instruction >> 16) & 15) as usize;
        let rn = ((instruction >> 12) & 15) as usize;
        let rs = ((instruction >> 8) & 15) as usize;
        let rm = (instruction & 15) as usize;
        let accumulate = instruction & (1 << 21) != 0;
        if rd == 15 || rs == 15 || rm == 15 || (accumulate && rn == 15) {
            return false;
        }
        let multiplier = self.registers[rs];
        let mut result = self.registers[rm].wrapping_mul(multiplier);
        if accumulate {
            result = result.wrapping_add(self.registers[rn]);
        }
        self.registers[rd] = result;
        if instruction & (1 << 20) != 0 {
            self.cpsr = (self.cpsr & !(CPSR_N | CPSR_Z))
                | if result >> 31 != 0 { CPSR_N } else { 0 }
                | if result == 0 { CPSR_Z } else { 0 };
        }
        let iterations = [8_u32, 16, 24]
            .into_iter()
            .find(|bits| {
                let upper = (multiplier as i32) >> bits;
                upper == 0 || upper == -1
            })
            .map_or(4, |bits| bits / 8);
        bus.idle(u64::from(iterations) + u64::from(accumulate));
        true
    }

    /// SWP snapshots the source before the read so overlapping operands retain
    /// the stored value. The word read rotates; the write aligns down.
    fn execute_swap<B: CpuBus>(
        &mut self,
        instruction: u32,
        bus: &mut B,
    ) -> Result<bool, CoreError> {
        if instruction & 0x0fb0_0ff0 != 0x0100_0090 {
            return Ok(false);
        }
        let target = self.registers[((instruction >> 16) & 15) as usize];
        let source = self.registers[(instruction & 15) as usize];
        let rd = ((instruction >> 12) & 15) as usize;
        let access = Access {
            kind: AccessKind::Data,
            sequential: false,
        };
        self.registers[rd] = if instruction & (1 << 22) != 0 {
            let value = bus.read8(target, access)?;
            bus.write8(target, source as u8, access)?;
            u32::from(value)
        } else {
            let value = bus
                .read32(target & !3, access)?
                .rotate_right((target & 3) * 8);
            bus.write32(store_address(target, 3), source, access)?;
            value
        };
        bus.idle(1);
        Ok(true)
    }

    /// Handles immediate and shifted-register byte/word transfers. Word loads
    /// rotate unaligned data; stores align down. Load/base overlap suppresses
    /// writeback, and a loaded PC retains state while refilling the pipeline.
    fn execute_word_transfer<B: CpuBus>(
        &mut self,
        address: u32,
        instruction: u32,
        bus: &mut B,
    ) -> Result<bool, CoreError> {
        if instruction & 0x0C00_0000 != 0x0400_0000 {
            return Ok(false);
        }
        let byte = instruction & (1 << 22) != 0;
        let rn = ((instruction >> 16) & 15) as usize;
        let rd = ((instruction >> 12) & 15) as usize;
        let pre = instruction & (1 << 24) != 0;
        let writeback = instruction & (1 << 21) != 0 || !pre;
        let load = instruction & (1 << 20) != 0;
        if writeback && rn == 15 {
            return Ok(false);
        }
        let base = if rn == 15 {
            address.wrapping_add(8)
        } else {
            self.registers[rn]
        };
        let offset = if instruction & (1 << 25) != 0 {
            if instruction & (1 << 4) != 0 {
                return Ok(false);
            }
            let rm = (instruction & 15) as usize;
            let value = if rm == 15 {
                address.wrapping_add(8)
            } else {
                self.registers[rm]
            };
            self.shift_immediate(value, (instruction >> 5) & 3, (instruction >> 7) & 31)
                .0
        } else {
            instruction & 0xfff
        };
        let updated = if instruction & (1 << 23) != 0 {
            base.wrapping_add(offset)
        } else {
            base.wrapping_sub(offset)
        };
        let target = if pre { updated } else { base };
        let access = Access {
            kind: AccessKind::Data,
            sequential: false,
        };
        if load {
            self.registers[rd] = if byte {
                u32::from(bus.read8(target, access)?)
            } else {
                bus.read32(target & !3, access)?
                    .rotate_right((target & 3) * 8)
            };
            bus.idle(1);
        } else {
            let value = if rd == 15 {
                address.wrapping_add(12)
            } else {
                self.registers[rd]
            };
            if byte {
                bus.write8(target, value as u8, access)?;
            } else {
                bus.write32(store_address(target, 3), value, access)?;
            }
        }
        if writeback && !(load && rn == rd) {
            self.registers[rn] = updated;
        }
        if load && rd == 15 {
            self.branch(bus, self.registers[15], false)?;
        }
        Ok(true)
    }

    /// User-bank transfers bypass only registers banked by the active mode.
    fn read_user_register(&self, register: usize) -> u32 {
        if (8..15).contains(&register)
            && (Self::status_bank(self.cpsr) == Some(0)
                || (register >= 13 && Self::status_bank(self.cpsr).is_some()))
        {
            self.user_bank[register - 8]
        } else {
            self.registers[register]
        }
    }

    /// Writes the user bank without changing CPSR or the currently visible bank.
    fn write_user_register(&mut self, register: usize, value: u32) {
        if (8..15).contains(&register)
            && (Self::status_bank(self.cpsr) == Some(0)
                || (register >= 13 && Self::status_bank(self.cpsr).is_some()))
        {
            self.user_bank[register - 8] = value;
        } else {
            self.registers[register] = value;
        }
    }

    /// Shares ARM/Thumb block-transfer semantics: ascending register order,
    /// aligned bus beats with unaligned base writeback, ARM7 empty-list behavior,
    /// and user-bank transfers or SPSR restoration when the S bit is present.
    fn execute_block_transfer<B: CpuBus>(
        &mut self,
        address: u32,
        instruction: u32,
        bus: &mut B,
    ) -> Result<bool, CoreError> {
        if instruction & 0x0E00_0000 != 0x0800_0000 {
            return Ok(false);
        }
        let rn = ((instruction >> 16) & 15) as usize;
        let raw_list = instruction & 0xffff;
        let list = if raw_list == 0 { 1 << 15 } else { raw_list };
        let user = instruction & (1 << 22) != 0;
        let load = instruction & (1 << 20) != 0;
        let writeback = instruction & (1 << 21) != 0;
        if rn == 15 {
            return Ok(false);
        }
        let bytes = if raw_list == 0 {
            64
        } else {
            list.count_ones() * 4
        };
        let base = self.registers[rn];
        let up = instruction & (1 << 23) != 0;
        let pre = instruction & (1 << 24) != 0;
        let updated = if up {
            base.wrapping_add(bytes)
        } else {
            base.wrapping_sub(bytes)
        };
        let mut target = if up {
            base.wrapping_add(if pre { 4 } else { 0 })
        } else {
            updated.wrapping_add(if pre { 0 } else { 4 })
        };
        let mut sequential = false;
        for register in 0..16 {
            if list & (1 << register) == 0 {
                continue;
            }
            let access = Access {
                kind: AccessKind::Data,
                sequential,
            };
            if load {
                let value = bus.read32(target & !3, access)?;
                if user && list & (1 << 15) == 0 {
                    self.write_user_register(register, value);
                } else {
                    self.registers[register] = value;
                }
            } else {
                let value = if register == 15 {
                    address.wrapping_add(if self.cpsr & CPSR_T != 0 { 6 } else { 12 })
                } else if register == rn && writeback && list.trailing_zeros() as usize != rn {
                    updated
                } else if user {
                    self.read_user_register(register)
                } else {
                    self.registers[register]
                };
                bus.write32(store_address(target, 3), value, access)?;
            }
            target = target.wrapping_add(4);
            sequential = true;
        }
        if writeback && !(load && list & (1 << rn) != 0) {
            self.registers[rn] = updated;
        }
        if load {
            bus.idle(1);
            if list & (1 << 15) != 0 {
                if user && let Some(bank) = Self::status_bank(self.cpsr) {
                    self.set_status(self.saved_status[bank]);
                }
                self.branch(bus, self.registers[15], false)?;
            }
        }
        Ok(true)
    }

    /// Routes immediate/register-offset LDRH/STRH through the timed bus.
    /// Register offsets are unshifted, as required by ARM halfword transfers.
    fn execute_halfword<B: CpuBus>(
        &mut self,
        address: u32,
        instruction: u32,
        bus: &mut B,
    ) -> Result<bool, CoreError> {
        if instruction & 0x0e00_0090 != 0x0000_0090 || instruction & 0x60 == 0 {
            return Ok(false);
        }
        let pre = instruction & (1 << 24) != 0;
        let writeback = instruction & (1 << 21) != 0;
        let base_register = ((instruction >> 16) & 15) as usize;
        let source = ((instruction >> 12) & 15) as usize;
        if source == 15 || ((!pre || writeback) && base_register == 15) || (!pre && writeback) {
            return Ok(false);
        }
        let base = if base_register == 15 {
            address.wrapping_add(8)
        } else {
            self.registers[base_register]
        };
        let offset = if instruction & (1 << 22) != 0 {
            ((instruction >> 4) & 0xF0) | (instruction & 15)
        } else {
            if instruction & 0xF00 != 0 || instruction & 15 == 15 {
                return Ok(false);
            }
            self.registers[(instruction & 15) as usize]
        };
        let updated = if instruction & (1 << 23) != 0 {
            base.wrapping_add(offset)
        } else {
            base.wrapping_sub(offset)
        };
        let access = Access {
            kind: AccessKind::Data,
            sequential: false,
        };
        let target = if pre { updated } else { base };
        if instruction & (1 << 20) != 0 {
            self.registers[source] =
                self.load_halfword_signed(bus, target, (instruction >> 5) & 3, access)?;
            bus.idle(1);
        } else {
            bus.write16(
                store_address(target, 1),
                self.registers[source] as u16,
                access,
            )?;
        }
        if (!pre || writeback) && !(instruction & (1 << 20) != 0 && base_register == source) {
            self.registers[base_register] = updated;
        }
        Ok(true)
    }

    /// ARM7 odd LDRH rotates the zero-extended halfword; odd LDRSH becomes
    /// LDRSB. Both instruction sets share these data-bus semantics.
    fn load_halfword_signed<B: CpuBus>(
        &self,
        bus: &mut B,
        target: u32,
        kind: u32,
        access: Access,
    ) -> Result<u32, CoreError> {
        match kind {
            1 => Ok(u32::from(bus.read16(target & !1, access)?).rotate_right((target & 1) * 8)),
            2 => Ok(bus.read8(target, access)? as i8 as i32 as u32),
            3 if target & 1 != 0 => Ok(bus.read8(target, access)? as i8 as i32 as u32),
            3 => Ok(bus.read16(target, access)? as i16 as i32 as u32),
            _ => Err(CoreError::InvalidTransferKind { kind }),
        }
    }
}

pub(super) struct StepOutcome {
    pub(super) address: u32,
}

/// Ordinary stores align to their transfer width. The cartridge backup bus
/// instead needs the original low address bits to select one source byte.
fn store_address(address: u32, alignment_mask: u32) -> u32 {
    if (0x0e000000..0x10000000).contains(&address) {
        address
    } else {
        address & !alignment_mask
    }
}

use crate::libs::Definitions::Arch::OP::InstructionType;
use crate::libs::Definitions::Arch::{RegNames, OP};
use crate::libs::Definitions::Errors::{ExecutionError, RegisterError};
use crate::libs::Definitions::Utils;
use crate::libs::Devices::Registers::{Available, Registers, SuccessfulOwn, HI_IDENT, LO_IDENT};
use crate::libs::Pipeline::Pipelined::Pipelined;
use crate::libs::Pipeline::Stages::Execution::EX_OUT;
use crate::libs::Pipeline::Stages::InstructionDecode::ReleaseBcast::{
    Freed, FreedWithContent, FreedWithContentDouble,
};
use crate::to_signed;
use std::error::Error;

pub struct InstructionDecode {
    pub latch_out_new_pc: u32,
    pub latch_out_A: Option<Result<Available, RegisterError>>,
    pub latch_out_B: Option<Result<Available, RegisterError>>,
    pub latch_out_I: Option<u32>,
    pub latch_out_RDest: Option<Result<SuccessfulOwn, RegisterError>>,
    pub latch_out_instruction_ID: usize,
    pub control_out_EXOP: (InstructionType, u32, u32),
    pub latch_in_new_pc: u32,
    pub latch_in_IR: (u32, u32),
    pub latch_in_RDest: Option<SuccessfulOwn>, // by this point it HAS to be a successful own, and RegisterError is not allowed
    pub latch_in_WB_contents: EX_OUT,
    pub latch_in_instruction_ID: usize,
    pub control_out_reg_release_bcast: ReleaseBcast,
    verbose: bool,
    pub regs: Registers,
    timestamp: usize,
}

#[derive(Debug)]
pub enum ReleaseBcast {
    NoRelease,
    Freed(usize, u32),
    FreedWithContent(usize, u32, u32),
    FreedWithContentDouble(usize, u32, u32),
}

impl Pipelined for InstructionDecode {
    fn tick(&mut self) -> Result<(), ExecutionError> {
        //ReleaseBcast step
        match self.latch_in_WB_contents {
            EX_OUT::AwaitingLock(owner, regno) => {
                panic!("[WB->ID]::Internal : Locked RDest register")
            }
            EX_OUT::Value(v) => {
                let register_number = match self.latch_in_RDest {
                    None => {
                        panic!("[WB->ID]::Internal : None as RDest!")
                    }
                    Some(own) => own.register_number,
                };
                self.regs
                    .write_and_unlock(register_number, v, self.latch_in_instruction_ID)?;
                self.control_out_reg_release_bcast =
                    FreedWithContent(self.latch_in_instruction_ID, register_number, v)
            }
            EX_OUT::DoubleValue(hi, lo) => {
                self.regs
                    .write_and_unlock(HI_IDENT, hi, self.latch_in_instruction_ID)?;
                self.regs
                    .write_and_unlock(LO_IDENT, lo, self.latch_in_instruction_ID)?;
                self.control_out_reg_release_bcast =
                    FreedWithContentDouble(self.latch_in_instruction_ID, hi, lo)
            }
            EX_OUT::DoJumpWithRA(_jtarg, rp) => {
                self.regs.write_and_unlock(
                    RegNames::RA as u32,
                    rp,
                    self.latch_in_instruction_ID,
                )?;
                self.control_out_reg_release_bcast =
                    FreedWithContent(self.latch_in_instruction_ID, RegNames::RA as u32, rp)
            }
            EX_OUT::Move(contents, destination) => {
                self.regs
                    .write_and_unlock(destination, contents, self.latch_in_instruction_ID)?;
                self.control_out_reg_release_bcast =
                    FreedWithContent(self.latch_in_instruction_ID, destination, contents)
            }

            _ => {}
        };

        let id = self.timestamp;
        self.timestamp = self.timestamp.wrapping_add(1);

        if self.verbose {
            println!("[ID@{id}]:\n\tDecoding 0x{:08x}", self.latch_in_IR.0);
        }
        //propagate latch signals
        self.latch_out_new_pc = self.latch_in_new_pc;

        let OP = (self.latch_in_IR.0 & 0xfc000000) >> 26;

        let code = self.latch_in_IR.0;
        //These handoffs should internally write to the out latches
        if OP == 0 {
            self.handoff_R(code, id)
        } else if !(OP == 0b000010 || OP == 0b000011 || OP == 0b011010) {
            self.handoff_I(code, id)
        } else {
            self.handoff_J(code)
        }

        self.latch_out_instruction_ID = id;
        Ok(())
    }
}

impl InstructionDecode {
    pub fn new(regs: Registers, verbose: bool) -> InstructionDecode {
        InstructionDecode {
            latch_out_new_pc: 0,
            latch_out_A: None,
            latch_out_B: None,
            latch_out_I: None,
            latch_out_RDest: None,
            latch_out_instruction_ID: 0,
            control_out_EXOP: (InstructionType::Special, 0, 0),
            latch_in_new_pc: 0,
            latch_in_IR: (0, 0),
            latch_in_RDest: Some(SuccessfulOwn { register_number: 0 }),
            latch_in_WB_contents: EX_OUT::NoOutput,
            latch_in_instruction_ID: 0,
            control_out_reg_release_bcast: ReleaseBcast::NoRelease,
            verbose,
            regs,
            timestamp: 0,
        }
    }

    fn handoff_R(&mut self, code: u32, accessor_id: usize) {
        let func = code & 0b00000000000000000000000000111111;
        let emitted_at = self.latch_in_IR.1;

        if code == OP::NOP {
            self.latch_out_A = None;
            self.latch_out_B = None;
            self.latch_out_RDest = None;
            self.control_out_EXOP = (InstructionType::Special, func, emitted_at);
        }

        let rs_regno = (code & 0b00000011111000000000000000000000) >> 21;
        let rt_regno = (code & 0b00000000000111110000000000000000) >> 16;
        let rd_regno = (code & 0b00000000000000001111100000000000) >> 11;

        let rs = self.regs.fetch(rs_regno);
        let rt = self.regs.fetch(rt_regno);
        let rd = self.regs.lock_for_write(rd_regno, accessor_id);

        let sham = (code & 0b00000000000000000000011111000000) >> 6;

        if self.verbose {
            println!(
                "\tR-type: func={:02x} rs={:?} rt={:?} rd={} sham={};",
                func, rs, rt, rd_regno, sham
            );
        }

        self.latch_out_A = Some(rs);
        self.latch_out_B = Some(rt);
        self.latch_out_I = Some(sham);
        self.latch_out_RDest = Some(rd);
        self.control_out_EXOP = (InstructionType::R, func, emitted_at);
    }

    fn handoff_I(&mut self, code: u32, accessor_id: usize) {
        let emitted_at = self.latch_in_IR.1;

        if code == OP::RFE || code == OP::HLT {
            if self.verbose {
                println!("\tSpecial::{}", if code == OP::RFE { "RFE" } else { "HLT" })
            }
            self.control_out_EXOP = (InstructionType::Special, code, emitted_at);
            return;
        };

        let rs_regno = (code & 0b00000011111000000000000000000000) >> 21;
        let rt_regno = (code & 0b00000000000111110000000000000000) >> 16;

        let rt = self.regs.lock_for_write(rt_regno, accessor_id);
        let rs = self.regs.fetch(rs_regno);

        let imm = code & 0b00000000000000001111111111111111;
        let func = (code & 0b11111100000000000000000000000000) >> 26;

        if self.verbose {
            let imm_sign_positive = (code & 0b00000000000000001000000000000000) == 0;
            println!(
                "\tI-type: func=0x{:02x} rs={:?} rt={:?} imm={}{};",
                func,
                rs,
                rt,
                if imm_sign_positive { "+" } else { "-" },
                to_signed!(imm, u16),
            );
        }

        self.latch_out_A = Some(rs);
        self.latch_out_B = Some(self.regs.fetch(rt_regno)); // Used for I-type branch instructions
        self.latch_out_I = Some(imm);
        self.latch_out_RDest = Some(rt);
        self.control_out_EXOP = (InstructionType::I, func, emitted_at);
    }

    fn handoff_J(&mut self, code: u32) {
        let emitted_at = self.latch_in_IR.1;

        if code == OP::SYSCALL {
            if self.verbose {
                println!(
                    "\tSyscall; v0={:?}, v1={:?}",
                    self.regs.fetch(RegNames::V0 as u32),
                    self.regs.fetch(RegNames::V1 as u32)
                )
            }

            self.control_out_EXOP = (InstructionType::Special, code, emitted_at);
        }

        let func = (code & 0xfc000000) >> 26;
        let jump_target = (code & !0xfc000000) << 2;

        if self.verbose {
            println!(
                "\tJ-type: func=0x{:02x} jump_target=0x{:08x}",
                func, jump_target
            )
        }

        self.latch_out_A = None;
        self.latch_out_B = None;
        self.latch_out_I = Some(jump_target);
        self.latch_out_RDest = None;
        self.control_out_EXOP = (InstructionType::J, code, emitted_at);
    }
}

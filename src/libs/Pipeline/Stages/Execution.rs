use crate::libs::Definitions::Arch;
use crate::libs::Definitions::Arch::OP;
use crate::libs::Definitions::Arch::OP::InstructionType;
use crate::libs::Definitions::Errors::{ExecutionError, RegisterError};
use crate::libs::Devices::Registers;
use crate::libs::Devices::Registers::{Available, SuccessfulOwn};
use crate::libs::Pipeline::Pipelined::Pipelined;
use crate::libs::Pipeline::Stages::Execution::EX_OUT::{
    AwaitingLock, DoJump, DoJumpWithRA, DoubleValue, LoadFrom, Move, NoOutput, StoreValue, Value,
};
use crate::{to_signed, to_signed_cond};

pub struct Execution {
    pub latch_in_new_pc: u32,
    pub latch_in_A: Option<Result<Available, RegisterError>>,
    pub latch_in_B: Option<Result<Available, RegisterError>>,
    pub latch_in_I: Option<u32>,
    pub latch_in_RDest: Option<Result<SuccessfulOwn, RegisterError>>,
    pub latch_in_instruction_ID: usize,
    pub control_in_EXOP: (InstructionType, u32, u32),
    pub control_in_free_register_feed: usize,
    pub latch_out_cond: bool,
    pub latch_out_EX_OUT: EX_OUT,
    pub latch_out_RDest: Option<Result<SuccessfulOwn, RegisterError>>,
    pub latch_out_instruction_ID: usize,
    pub latch_out_is_privileged: bool, // NOT ONLY LATCH OUT, also used for internal privileges
    pub control_out_termination: bool,
    verbose: bool,
    EPC: u32,
    irq_handler_addr: u32,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum EX_OUT {
    AwaitingLock(Option<(usize, u32)>, Option<(usize, u32)>),
    Value(u32),
    DoubleValue(u32, u32),
    LoadFrom(u32, u32),
    StoreValue(u32, u32, u32),
    DoJump(u32),
    DoJumpWithRA(u32, u32),
    Move(u32, u32),
    NoOutput,
    Abort,
}

impl Pipelined for Execution {
    fn tick(&mut self) -> Result<(), ExecutionError> {
        //TODO: Where are interrupts caused? in IF? here?

        if self.verbose {
            println!(
                "[EX@{}]:\n\tReceived {:?}-type instruction with function code 0x{:02x}",
                self.latch_in_instruction_ID, self.control_in_EXOP.0, self.control_in_EXOP.1
            )
        }

        //TODO: Sign propagation on signed operations (loads/stores, general operations, etc)

        let output = match self.control_in_EXOP.0 {
            InstructionType::I => {
                let A = match self.latch_in_A {
                    Some(A) => A,
                    None => {
                        panic!(
                            "[EX]::Internal : Received malformed A for I-type instruction {:?}",
                            self.control_in_EXOP
                        )
                    }
                };
                let B = match self.latch_in_B {
                    Some(B) => B,
                    None => {
                        panic!(
                            "[EX]::Internal : Received malformed A for I-type instruction {:?}",
                            self.control_in_EXOP
                        )
                    }
                };
                let imm = match self.latch_in_I {
                    Some(I) => I,
                    None => {
                        panic!(
                            "[EX]::Internal : Received malformed Imm for I-type instruction {:?}",
                            self.control_in_EXOP
                        )
                    }
                };
                let _verify_valid_rdest = match self.latch_in_RDest {
                    None => {
                        panic!(
                            "[EX]::Internal : Received None Rdest for R-type instruction {:?}",
                            self.control_in_EXOP
                        )
                    }
                    Some(_) => {}
                };
                self.handoff_I(self.control_in_EXOP.1, A, B, imm, self.control_in_EXOP.2)?
            }
            InstructionType::R => {
                let A = match self.latch_in_A {
                    Some(A) => A,
                    None => {
                        panic!(
                            "[EX]::Internal : Received malformed A for R-type instruction {:?}",
                            self.control_in_EXOP
                        )
                    }
                };
                let B = match self.latch_in_B {
                    Some(B) => B,
                    None => {
                        panic!(
                            "[EX]::Internal : Received malformed A for I-type instruction {:?}",
                            self.control_in_EXOP
                        )
                    }
                };
                let sham = match self.latch_in_I {
                    Some(I) => I,
                    None => {
                        panic!(
                            "[EX]::Internal : Received malformed Imm for I-type instruction {:?}",
                            self.control_in_EXOP
                        )
                    }
                };
                let _verify_valid_rdest = match self.latch_in_RDest {
                    None => {
                        panic!(
                            "[EX]::Internal : Received None Rdest for R-type instruction {:?}",
                            self.control_in_EXOP
                        )
                    }
                    Some(_) => {}
                };

                self.handoff_R(self.control_in_EXOP.1, A, B, sham, self.control_in_EXOP.2)?
            }
            InstructionType::J => {
                let imm = match self.latch_in_I {
                    None => {
                        panic!(
                            "[EX]::Internal : Received malformed Imm for J-type instruction {:?}",
                            self.control_in_EXOP
                        )
                    }
                    Some(I) => I,
                };
                self.handoff_J(self.control_in_EXOP.1, imm, self.control_in_EXOP.2)?
            }
            InstructionType::Special => {
                self.handoff_special(self.control_in_EXOP.1, self.control_in_EXOP.2)?
            }
        };

        self.latch_out_RDest = self.latch_in_RDest;
        self.latch_out_EX_OUT = output;
        //if the output is DoJump, set cond
        //OR if we are stalling, clear outs
        match output {
            DoJump(_) | DoJumpWithRA(_, _) => {
                self.latch_out_cond = true;
                if self.verbose {
                    println!("\tEnforcing jump : {:?}", output)
                }
            }
            AwaitingLock(_, _) => {
                self.latch_out_instruction_ID = 0;
                self.latch_out_cond = false;
                self.latch_out_RDest = None;
            }
            _ => self.latch_out_cond = false,
        };
        self.latch_out_instruction_ID = self.latch_in_instruction_ID;
        Ok(())
    }
}

impl Execution {
    pub fn new(irq_handler_addr: u32, verbose: bool) -> Execution {
        Execution {
            latch_in_new_pc: 0,
            latch_in_A: None,
            latch_in_B: None,
            latch_in_I: None,
            latch_in_RDest: None,
            latch_in_instruction_ID: 0,
            control_in_EXOP: (InstructionType::Special, 0, 0),
            control_in_free_register_feed: 0,
            latch_out_cond: false,
            latch_out_EX_OUT: NoOutput,
            latch_out_RDest: None,
            latch_out_instruction_ID: 0,
            control_out_termination: false,
            verbose,
            latch_out_is_privileged: false,
            EPC: 0,
            irq_handler_addr,
        }
    }

    fn handoff_I(
        &mut self,
        operation: u32,
        operand_1: Result<Available, RegisterError>,
        operand_2: Result<Available, RegisterError>,
        immediate: u32,
        emitted_at: u32,
    ) -> Result<EX_OUT, ExecutionError> {
        // Await possibly busy values

        let mut A = 0;
        let A_lock;
        match operand_1 {
            Ok(Available { value: v }) => {
                A = v;
                A_lock = None;
            }
            Err(RegisterError::LockedWithHandle(handle, regno)) => {
                A_lock = Some((handle, regno));
            }
            e => {
                panic!("[EX]::Internal : Received a malformed A: {:?}", e)
            }
        };

        let mut B = 0;
        let B_lock;
        match operand_2 {
            Ok(Available { value: v }) => {
                B = v;
                B_lock = None
            }
            Err(RegisterError::LockedWithHandle(handle, regno)) => {
                B_lock = Some((handle, regno));
            }
            e => {
                panic!("[EX]::Internal : Received a malformed B: {:?}", e)
            }
        };

        if A_lock.is_some() {
            // 'stall', clear rdest
            self.latch_out_RDest = None;
            return Ok(AwaitingLock(A_lock, B_lock));
        }
        //B only used as source register for compare branch instructions
        match operation {
            OP::I::BNE | OP::I::BEQ => {
                if B_lock.is_some() {
                    // 'stall', clear rdest
                    self.latch_out_RDest = None;
                    return Ok(AwaitingLock(A_lock, B_lock));
                }
            }
            _ => {}
        }

        let imm_sign_positive = (immediate & 0b00000000000000001000000000000000) == 0;
        let out: EX_OUT = match operation {
            OP::I::ADDI => {
                //using signed, if number is negative subtract
                if imm_sign_positive {
                    Value(A.overflowing_add(immediate).0)
                } else {
                    Value(A.overflowing_sub(immediate).0)
                }
            }
            OP::I::ADDIU => Value(A.overflowing_add(immediate).0),
            OP::I::ANDI => Value(A & immediate),
            OP::I::ORI => Value(A | immediate),
            OP::I::XORI => Value(A ^ immediate),
            OP::I::SLTI => {
                if A < immediate {
                    Value(1)
                } else {
                    Value(0)
                }
            }
            OP::I::SLTIU => {
                if A < immediate {
                    Value(1)
                } else {
                    Value(0)
                }
            }
            OP::I::LHI => {
                todo!("lhi");
            }
            OP::I::LLO => {
                todo!("llo");
            }
            OP::I::BEQ => {
                if A == B {
                    if imm_sign_positive {
                        DoJump(emitted_at.overflowing_add(immediate << 2).0 + 4)
                    } else {
                        DoJump(
                            emitted_at
                                .overflowing_sub(to_signed!(immediate << 2, u16) + 4)
                                .0,
                        )
                    }
                } else {
                    NoOutput
                }
            }
            OP::I::BNE => {
                if A != B {
                    if imm_sign_positive {
                        DoJump(emitted_at.overflowing_add(immediate << 2).0)
                    } else {
                        DoJump(
                            emitted_at
                                .overflowing_sub(to_signed!(immediate << 2, u16))
                                .0,
                        )
                    }
                } else {
                    NoOutput
                }
            }
            OP::I::BGTZ => {
                if A > 0 {
                    if imm_sign_positive {
                        DoJump(emitted_at.overflowing_add(immediate << 2).0)
                    } else {
                        DoJump(
                            emitted_at
                                .overflowing_sub(to_signed!(immediate << 2, u16))
                                .0,
                        )
                    }
                } else {
                    NoOutput
                }
            }
            OP::I::BLEZ => {
                if A <= B {
                    if imm_sign_positive {
                        DoJump(emitted_at.overflowing_add(immediate << 2).0)
                    } else {
                        DoJump(
                            emitted_at
                                .overflowing_sub(to_signed!(immediate << 2, u16))
                                .0,
                        )
                    }
                } else {
                    NoOutput
                }
            }
            OP::I::LB => LoadFrom(A + immediate, 1),
            OP::I::LBU => LoadFrom(A + immediate, 1),
            OP::I::LH => LoadFrom(A + immediate, 2),
            OP::I::LHU => LoadFrom(A + immediate, 2),
            OP::I::LW => LoadFrom(A + immediate, 4),
            OP::I::SB => StoreValue(B, A + immediate, 1),
            OP::I::SH => StoreValue(B, A + immediate, 2),
            OP::I::SW => StoreValue(B, A + immediate, 4),

            _ => {
                return Err(ExecutionError::UnrecognizedOPError(format!(
                    "Unrecognized I type func {:x}",
                    operation
                )))
            }
        };

        if self.verbose {
            println!("\tOutputted {:?}", out);
        }

        Ok(out)
    }

    fn handoff_R(
        &mut self,
        operation: u32,
        operand_1: Result<Available, RegisterError>,
        operand_2: Result<Available, RegisterError>,
        sham: u32,
        emitted_at: u32,
    ) -> Result<EX_OUT, ExecutionError> {
        // Await possibly busy values

        let mut A = 0;
        let A_lock;
        match operand_1 {
            Ok(Available { value: v }) => {
                A = v;
                A_lock = None;
            }
            Err(RegisterError::LockedWithHandle(handle, regno)) => {
                A_lock = Some((handle, regno));
            }
            e => {
                panic!("[EX]::Internal : Received a malformed A: {:?}", e)
            }
        };

        let mut B = 0;
        let B_lock;
        match operand_2 {
            Ok(Available { value: v }) => {
                B = v;
                B_lock = None
            }
            Err(RegisterError::LockedWithHandle(handle, regno)) => {
                B_lock = Some((handle, regno));
            }
            e => {
                panic!("[EX]::Internal : Received a malformed B: {:?}", e)
            }
        };

        if A_lock.is_some() || B_lock.is_some() {
            // 'stall', clear rdest
            self.latch_out_RDest = None;
            return Ok(AwaitingLock(A_lock, B_lock));
        }

        let A_sign_positive = (A & 0x80000000) == 0;
        let B_sign_positive = (B & 0x80000000) == 0;

        let out: EX_OUT = match operation {
            OP::R::ADD => {
                if A_sign_positive {
                    Value(A.overflowing_add(B).0)
                } else {
                    Value(A.overflowing_sub(B).0)
                }
            }
            OP::R::ADDU => Value(A.overflowing_add(B).0),
            OP::R::AND => Value(A & B),
            OP::R::NOR => Value(!(A | B)),
            OP::R::OR => Value(A | B),
            OP::R::SUB => Value(A.overflowing_sub(B).0),
            OP::R::SUBU => Value(A.overflowing_sub(B).0),
            OP::R::XOR => Value(A ^ B),
            OP::R::SLT => {
                if to_signed_cond!(A, A_sign_positive, u32)
                    < to_signed_cond!(A, A_sign_positive, u32)
                {
                    Value(1)
                } else {
                    Value(0)
                }
            }
            OP::R::SLTU => {
                if A < B {
                    Value(1)
                } else {
                    Value(0)
                }
            }
            OP::R::DIV => {
                let lo = A / B;
                let hi = A % B;
                DoubleValue(hi, lo)
            }
            OP::R::DIVU => {
                let lo = A.saturating_div(A);
                let hi = A % B;
                DoubleValue(hi, lo)
            }
            OP::R::MULT => {
                let (hi, lo) = (to_signed_cond!(A, A_sign_positive, u32))
                    .widening_mul(to_signed_cond!(B, B_sign_positive, u32));
                DoubleValue(hi, lo)
            }
            OP::R::MULTU => {
                let (hi, lo) = A.widening_mul(B);
                DoubleValue(hi, lo)
            }
            OP::R::SLL => Value(A << sham),
            OP::R::SRA => {
                Value((A as i32 >> sham as i32) as u32)
                //sra, srav, srlv, etc : for rust to do shift aritmetic, use signed types
            }
            OP::R::SRAV => Value((A as i32 >> B as i32) as u32),
            OP::R::SRLV => Value(A >> B),
            OP::R::JALR => {
                let jump_address = if A != 0 { A - 4 } else { A };
                DoJumpWithRA(jump_address, emitted_at)
            }
            OP::R::JR => {
                let jump_address = if A != 0 { A - 4 } else { A };
                DoJump(jump_address)
            }
            OP::R::MFHI => {
                let dest_regno = match &self.latch_out_RDest {
                    None => {
                        unreachable!()
                    }
                    Some(r) => match r {
                        Ok(SuccessfulOwn {
                            register_number: regno,
                        }) => regno,
                        Err(RegisterError::LockedWithHandle(handle, regno)) => regno,
                        Err(RegisterError::NotOwned(_, _)) => {
                            panic!("NotOwned reached EX as RDest!")
                        }
                    },
                };
                Move(Registers::HI_IDENT, *dest_regno)
            }
            OP::R::MFLO => {
                let dest_regno = match &self.latch_out_RDest {
                    None => {
                        unreachable!()
                    }
                    Some(r) => match r {
                        Ok(SuccessfulOwn {
                            register_number: regno,
                        }) => (regno),
                        Err(RegisterError::LockedWithHandle(handle, regno)) => regno,
                        Err(RegisterError::NotOwned(_, _)) => {
                            panic!("NotOwned reached EX as RDest!")
                        }
                    },
                };
                Move(Registers::LO_IDENT, *dest_regno)
            }
            OP::R::MTHI => Move(A, Registers::HI_IDENT),
            OP::R::MTLO => Move(A, Registers::LO_IDENT),

            _ => {
                return Err(ExecutionError::UnrecognizedOPError(format!(
                    "Unrecognized R type func {:x}",
                    operation
                )));
            }
        };

        Ok(out)
    }

    fn handoff_J(
        &mut self,
        operation: u32,
        immediate: u32,
        emitted_at: u32,
    ) -> Result<EX_OUT, ExecutionError> {
        let out = match operation {
            OP::J::J => {
                let jump_target = if immediate != 0 {
                    immediate - 4
                } else {
                    immediate
                };
                DoJump(jump_target)
            }
            OP::J::JAL => {
                let jump_target = if immediate != 0 {
                    immediate - 4
                } else {
                    immediate
                };
                DoJumpWithRA(jump_target, emitted_at)
            }

            _ => {
                return Err(ExecutionError::UnrecognizedOPError(format!(
                    "Unrecognized J type func {:02x}",
                    operation
                )));
            }
        };

        Ok(out)
    }

    fn handoff_special(
        &mut self,
        operation: u32,
        emitted_at: u32,
    ) -> Result<EX_OUT, ExecutionError> {
        let out = match operation {
            OP::SYSCALL => {
                self.EPC = emitted_at;
                if self.verbose {
                    println!("\tChanged privilege mode to true");
                }
                self.latch_out_is_privileged = true;
                Ok(DoJump(self.irq_handler_addr))
            }
            OP::NOP => {
                if self.verbose {
                    println!("\tNOP")
                }
                return Ok(NoOutput);
            }
            OP::RFE => {
                if self.verbose {
                    // If RFE is called, a previous instruction triggered an interrupt and the restore address should be present
                    println!(
                        "\tRFE: EPC=0x{:08x}; privilege status: {}",
                        self.EPC, self.latch_out_is_privileged
                    );
                }

                if !self.latch_out_is_privileged {
                    return Err(ExecutionError::PrivilegeError(String::from("RFE")));
                }

                // When RFE is called, the new PC is emitted as EX_OUT
                self.latch_out_is_privileged = false;

                if self.verbose {
                    println!("\tChanged privilege mode to false");
                }
                return Ok(DoJump(self.EPC));
            }
            OP::HLT => {
                if self.verbose {
                    println!("\tHLT: privilege status: {}", self.latch_out_is_privileged)
                }

                if !self.latch_out_is_privileged {
                    return Err(ExecutionError::PrivilegeError(String::from("HLT")));
                }

                //TODO: In non pipelined we close the interrupt channel. we havent decided how interrupts
                //are done here. Do we mantain the same API?

                self.latch_out_is_privileged = false;
                self.control_out_termination = true;

                return Ok(EX_OUT::Abort);
            }
            _ => {
                return Err(ExecutionError::UnrecognizedOPError(format!(
                    "Unrecognized Special type func {:02x}",
                    operation
                )));
            }
        };

        out
    }
}

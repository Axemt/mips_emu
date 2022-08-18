use crate::libs::Definitions::Errors::{ExecutionError, RegisterError};
use crate::libs::Definitions::Utils;
use crate::libs::Devices::Registers::{Available, SuccessfulOwn};
use crate::libs::Memory::Memory;
use crate::libs::Pipeline::PipelinedWithHeldMemory::PipelinedWithHeldMemory;
use crate::libs::Pipeline::Stages::Execution::EX_OUT;
use crate::libs::Pipeline::Stages::Execution::EX_OUT::{NoOutput, Value};
use std::borrow::BorrowMut;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

pub struct MemoryStage {
    pub latch_in_EX_OUT: EX_OUT,
    pub latch_in_RDest: Option<Result<SuccessfulOwn, RegisterError>>,
    pub latch_in_instruction_ID: usize,
    pub latch_out_WB: EX_OUT,
    pub latch_out_RDest: Option<Result<SuccessfulOwn, RegisterError>>,
    pub latch_out_instruction_ID: usize,
    pub is_privileged: bool,
    verbose: bool,
}

impl PipelinedWithHeldMemory for MemoryStage {
    fn tick_with_mem(&mut self, mem: &mut Memory) -> Result<(), ExecutionError> {
        if self.verbose {
            print!("[MEM@{}]:\n\t", self.latch_in_instruction_ID)
        }
        mem.set_privileged(self.is_privileged);
        match &self.latch_in_EX_OUT {
            EX_OUT::LoadFrom(addr, size) => {
                let val = mem.load(*addr, *size as usize)?;
                let out = if *size == 1 {
                    Value(Utils::from_byte(val))
                } else if *size == 2 {
                    Value(Utils::from_half(val))
                } else if *size == 4 {
                    Value(Utils::from_word(val))
                } else {
                    return Err(ExecutionError::MemError(format!(
                        "Size is not supported: Size={}",
                        size
                    )));
                };

                self.latch_out_WB = out;
            }
            EX_OUT::StoreValue(B, addr, size) => {
                if *size == 1 {
                    let v: Vec<u8> = vec![(*B & 0x000000ff) as u8; 1];
                    mem.store(*addr as usize, 1, &v)?;
                } else if *size == 2 {
                    let v = vec![(B >> 8) as u8, (B & 0x00ff) as u8];
                    mem.store(*addr as usize, 2, &v)?;
                } else if *size == 4 {
                    let v = vec![
                        (B & 0xff000000 >> 24) as u8,
                        (B & 0x00ff0000 >> 16) as u8,
                        (B & 0x0000ff00 >> 8) as u8,
                        (B & 0x000000ff) as u8,
                    ];
                    mem.store(*addr as usize, 4, &v)?;
                } else {
                    return Err(ExecutionError::MemError(format!(
                        "Size is not supported: Size={}",
                        size
                    )));
                }

                self.latch_out_WB = NoOutput; // consumed
            }

            _ => {
                self.latch_out_WB = self.latch_in_EX_OUT;
            }
        }
        self.latch_out_RDest = self.latch_in_RDest;
        self.latch_out_instruction_ID = self.latch_in_instruction_ID;

        Ok(())
    }
}

impl MemoryStage {
    pub fn new(verbose: bool) -> MemoryStage {
        MemoryStage {
            latch_in_EX_OUT: EX_OUT::NoOutput,
            latch_in_RDest: None,
            latch_in_instruction_ID: 0,
            latch_out_WB: EX_OUT::NoOutput,
            latch_out_RDest: None,
            latch_out_instruction_ID: 0,
            is_privileged: false,
            verbose,
        }
    }
}

use crate::libs::Definitions::Errors::ExecutionError;
use crate::libs::Definitions::Utils;
use crate::libs::Memory::Memory;
use crate::libs::Pipeline::Pipelined::Pipelined;
use crate::libs::Pipeline::PipelinedWithHeldMemory::PipelinedWithHeldMemory;
use std::cell::{Cell, RefCell, RefMut};

// IF contains a latch to the next stage with the new PC and the Instruction read but not decoded
// Passing of latch info is handled by the main loop of the pipelined Core
pub struct InstructionFetch {
    pub latch_out_new_pc: u32,
    pub latch_out_IR: (u32, u32),
    pub latch_in_cond: bool,
    pub latch_in_new_pc: u32,
    pub pc: u32,
    verbose: bool,
}

impl PipelinedWithHeldMemory for InstructionFetch {
    fn tick_with_mem(&mut self, mem: &mut Memory) -> Result<(), ExecutionError> {
        if self.verbose {
            print!("[IF]:\n\t")
        }
        let current_pc = self.pc;
        self.latch_out_IR = (Utils::from_word(mem.load(current_pc, 4)?), current_pc);
        if self.verbose {
            println!(
                "\tFetched code 0x{:08x?} at PC 0x{:08x}",
                self.latch_out_IR.0, self.pc
            )
        }

        if self.latch_in_cond {
            self.pc = self.latch_in_new_pc;
        } else {
            self.pc += 4;
        }

        self.latch_out_new_pc = self.pc;

        Ok(())
    }
}

impl InstructionFetch {
    pub fn new(start_pc: u32, verbose: bool) -> InstructionFetch {
        InstructionFetch {
            latch_out_new_pc: 0,
            latch_out_IR: (0x00000000, 0x00000000), //NOP
            latch_in_cond: false,
            latch_in_new_pc: 0,
            pc: start_pc,
            verbose,
        }
    }
}
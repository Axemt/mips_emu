use crate::libs::Definitions::Arch::RegNames;
use crate::libs::Definitions::Errors::{ExecutionError, RegisterError};
use crate::libs::Devices::Registers::SuccessfulOwn;
use crate::libs::Pipeline::Pipelined::Pipelined;
use crate::libs::Pipeline::Stages::Execution::EX_OUT;
use crate::libs::Pipeline::Stages::Execution::EX_OUT::NoOutput;

pub struct WriteBack {
    pub latch_in_WB: EX_OUT,
    pub latch_in_RDest: Option<Result<SuccessfulOwn, RegisterError>>,
    pub latch_in_instruction_ID: usize,
    pub latch_out_WB_contents: EX_OUT,
    pub latch_out_RDest: Option<SuccessfulOwn>,
    pub latch_out_instruction_ID: usize,
    verbose: bool,
}

impl Pipelined for WriteBack {
    fn tick(&mut self) -> Result<(), ExecutionError> {
        self.latch_out_WB_contents = self.latch_in_WB;
        if self.verbose {
            println!("[WB@{}]", self.latch_in_instruction_ID);
        }

        if let Some(res_own_reg) = self.latch_in_RDest {
            // RDest MUST be successfully owned when propagating or panic
            if res_own_reg.is_err() {
                panic!(
                    "[WB]::Internal : Register Error reached WB stage : {:?}",
                    res_own_reg.expect_err("")
                )
            }

            self.latch_out_RDest = Some(res_own_reg.expect(""));
        } else {
            self.latch_out_RDest = None;
        }

        if self.latch_in_WB != NoOutput && self.latch_in_RDest.is_some() {
            println!(
                "\tEmitted order to write back {:?} to destination register {:?}",
                self.latch_in_WB,
                self.latch_in_RDest.expect("").unwrap().register_number
            );
        } else {
            println!()
        }

        Ok(())
    }
}

impl WriteBack {
    pub fn new(verbose: bool) -> WriteBack {
        WriteBack {
            latch_in_WB: EX_OUT::NoOutput,
            latch_in_RDest: None,
            latch_in_instruction_ID: 0,
            latch_out_WB_contents: EX_OUT::NoOutput,
            latch_out_RDest: None,
            latch_out_instruction_ID: 0,
            verbose,
        }
    }
}

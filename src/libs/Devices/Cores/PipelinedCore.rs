use crate::libs::Definitions::Arch;
use crate::libs::Definitions::Arch::{RegNames, OP};
use crate::libs::Definitions::Utils::{Byte, Half, Word};
use crate::libs::Definitions::{Stats, Utils};
use crate::libs::Memory::Memory;
use std::io::Error;

use crate::libs::Definitions::Errors::{ExecutionError, HeaderError, RegisterError};

use crate::libs::Devices::{Console, Interruptor, Keyboard, MemoryMapped};

use crate::to_signed_cond;
use crate::{to_signed, Runnable};

use crate::libs::Definitions::Arch::OP::InstructionType;
use crate::libs::Devices::Cores::CoreTraits;
use crate::libs::Devices::Registers::{Available, Registers, SuccessfulOwn};
use crate::libs::Pipeline::Pipelined::Pipelined;
use crate::libs::Pipeline::PipelinedWithHeldMemory::PipelinedWithHeldMemory;
use crate::libs::Pipeline::Stages::Execution::EX_OUT::{
    AwaitingLock, DoJump, DoJumpWithRA, NoOutput,
};
use crate::libs::Pipeline::Stages::Execution::{Execution, EX_OUT};
use crate::libs::Pipeline::Stages::InstructionDecode::{InstructionDecode, ReleaseBcast};
use crate::libs::Pipeline::Stages::InstructionFetch::InstructionFetch;
use crate::libs::Pipeline::Stages::MemoryStage::MemoryStage;
use crate::libs::Pipeline::Stages::WriteBack::WriteBack;
use crate::CoreTraits::Privileged;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::{mpsc, Mutex};
use std::time::Duration;

pub struct Core {
    IF: InstructionFetch,
    ID: InstructionDecode,
    EX: Execution,
    MEM: MemoryStage,
    pub held_mem: Memory,
    WB: WriteBack,
    verbose: bool,
}

impl Core {
    pub fn new<'a>(verbose: bool) -> Core {
        let mut regs = Registers::new(verbose);
        let mut held_mem = Memory::new(verbose);

        //init default irq_handler
        held_mem.set_privileged(true);
        let DEFAULT_irq = Arch::DEFAULT_IRQH;
        let irq_handler_addr: u32 = 0x0;

        if verbose {
            println!(
                "[CORE]: Setting up default IRQH with address 0x{:08x}",
                irq_handler_addr
            )
        }

        let stackbase = DEFAULT_irq.len() as u32 + 8;
        held_mem
            .store(irq_handler_addr as usize, DEFAULT_irq.len(), &DEFAULT_irq)
            .unwrap();
        held_mem.protect(irq_handler_addr, stackbase - 4);

        if verbose {
            println!(
                "[CORE]: Setting up stack from 0x{:08x} to 0x{:08x}",
                stackbase,
                stackbase + Arch::STACKSIZE
            );
        }

        held_mem.protect(stackbase, stackbase + Arch::STACKSIZE);

        //regs is newly created, lock should never return Err
        regs.lock_for_write(RegNames::SP as u32, 0).unwrap();
        regs.write_and_unlock(RegNames::SP as u32, stackbase, 0)
            .expect("[CORE::Internal]: Constructor failed; success should be guaranteed");
        held_mem.set_privileged(false);

        //add basic mapped devices
        let console = Box::new(Console::new());
        let keyboard = Box::new(Keyboard::new());
        held_mem.map_device(console.range_lower, console.range_upper, console);
        held_mem.map_device(keyboard.range_lower, keyboard.range_upper, keyboard);

        // interrupts???
        //let (send, recv) = mpsc::channel();

        let start_pc = 0;
        let IF = InstructionFetch::new(start_pc, verbose);
        let ID = InstructionDecode::new(regs, verbose);
        let EX = Execution::new(irq_handler_addr, verbose);
        let MEM = MemoryStage::new(verbose);
        let WB = WriteBack::new(verbose);

        Core {
            IF,
            ID,
            EX,
            MEM,
            held_mem,
            WB,
            verbose,
        }
    }
}

impl Runnable for Core {
    fn run(&mut self) -> Result<(), ExecutionError> {
        let mut EX_IN_STALL;
        let mut stat = Stats::new(5);
        loop {
            EX_IN_STALL = false;
            if self.verbose {
                println!("--------------------");
            }

            if self.EX.control_out_termination {
                stat.mark_finished();
                if self.verbose {
                    println!("[CORE]: Termination Flag Set");
                }
                break;
                //TODO: when breaking, ensure instructions in MEM and WB have completed
            }

            //TODO: Interrupt stuff
            //TICK STEP:
            self.IF.tick_with_mem(&mut self.held_mem)?;
            self.ID.tick()?;

            //CONTROL : ReleaseBcast
            // if awaiting a lock on A and/or B, they are held in EX.latch_in_[A,B]
            // if awaiting a lock on RDest, it can be freed at any stage; entering EX, MEM or WB
            match self.ID.control_out_reg_release_bcast {
                ReleaseBcast::NoRelease => {}
                ReleaseBcast::Freed(id, regno) => {
                    if self.verbose && regno != 0 {
                        println!("[CORE]: Broadcasting Free of register {regno} owned by {id}")
                    }
                    //Freeing of RDest
                    if Some(Err(RegisterError::LockedWithHandle(id, regno)))
                        == self.WB.latch_in_RDest
                    {
                        self.WB.latch_in_RDest = Some(Ok(SuccessfulOwn {
                            register_number: regno,
                        }))
                    } else if Some(Err(RegisterError::LockedWithHandle(id, regno)))
                        == self.MEM.latch_in_RDest
                    {
                        self.MEM.latch_in_RDest = Some(Ok(SuccessfulOwn {
                            register_number: regno,
                        }))
                    } else if Some(Err(RegisterError::LockedWithHandle(id, regno)))
                        == self.EX.latch_in_RDest
                    {
                        self.EX.latch_in_RDest = Some(Ok(SuccessfulOwn {
                            register_number: regno,
                        }))
                    }

                    //clear hold values in the pipeline
                    if let AwaitingLock(optA, optB) = self.WB.latch_in_WB_contents {
                        if optA == Some((id, regno)) || optB == Some((id, regno)) {
                            self.WB.latch_in_WB_contents = NoOutput;
                        }
                        if self.verbose {
                            println!("[CORE]: Cleared AwaitingLock from WB_in");
                        }
                    }
                }
                ReleaseBcast::FreedWithContent(id, regno, v) => {
                    if self.verbose && regno != 0 {
                        println!("[CORE]: Broadcasting Free of register {regno} owned by {id} with content {v}")
                    }
                    // Freeing of A, B
                    if Some(Err(RegisterError::LockedWithHandle(id, regno))) == self.EX.latch_in_A {
                        self.EX.latch_in_A = Some(Ok(Available { value: v }))
                    }
                    if Some(Err(RegisterError::LockedWithHandle(id, regno))) == self.EX.latch_in_B {
                        self.EX.latch_in_B = Some(Ok(Available { value: v }))
                    }

                    //Freeing of RDest
                    if Some(Err(RegisterError::LockedWithHandle(id, regno)))
                        == self.WB.latch_in_RDest
                    {
                        self.WB.latch_in_RDest = Some(Ok(SuccessfulOwn {
                            register_number: regno,
                        }))
                    } else if Some(Err(RegisterError::LockedWithHandle(id, regno)))
                        == self.MEM.latch_in_RDest
                    {
                        self.MEM.latch_in_RDest = Some(Ok(SuccessfulOwn {
                            register_number: regno,
                        }))
                    } else if Some(Err(RegisterError::LockedWithHandle(id, regno)))
                        == self.EX.latch_in_RDest
                    {
                        self.EX.latch_in_RDest = Some(Ok(SuccessfulOwn {
                            register_number: regno,
                        }))
                    }

                    //clear hold values in the pipeline
                    if let AwaitingLock(optA, optB) = self.WB.latch_in_WB_contents {
                        if optA == Some((id, regno)) || optB == Some((id, regno)) {
                            self.WB.latch_in_WB_contents = NoOutput;
                        }
                        if self.verbose {
                            println!("[CORE]: Cleared AwaitingLock from WB_in");
                        }
                    }
                }
                ReleaseBcast::FreedWithContentDouble(_id, _hi, _lo) => {}
            }

            self.EX.tick()?;
            self.MEM.tick_with_mem(&mut self.held_mem)?;
            self.WB.tick()?;

            if self.verbose {
                println!()
            }

            //After ReleaseBcast, check if EX has to be stalled or not
            //Are we stalling in EX OR enforcing a jump?
            match self.EX.latch_out_EX_OUT {
                AwaitingLock(_A_lock, _B_lock) => {
                    if self.verbose {
                        println!(
                            "[CORE]: Stalling EX@{}; Reason: {:?}",
                            self.EX.latch_out_instruction_ID, self.EX.latch_out_EX_OUT
                        );
                    }
                    EX_IN_STALL = true;
                    self.ID.control_in_stall = true;
                    self.IF.control_in_stall = true;
                }
                DoJump(jtarg) | DoJumpWithRA(jtarg, _) => self.IF.latch_in_new_pc = jtarg,
                _ => {}
            }

            //PROPAGATION STEP

            //propagate: IF->ID : only if !EX_IS_STALL
            if !EX_IN_STALL {
                self.ID.latch_in_new_pc = self.IF.latch_out_new_pc;
                self.ID.latch_in_IR = self.IF.latch_out_IR;
            }
            //propagate: WB->ID
            self.ID.latch_in_RDest = self.WB.latch_out_RDest;
            self.ID.latch_in_WB_contents = self.WB.latch_out_WB_contents;

            //propagate: ID->EX : only if !EX_IS_STALL
            if !EX_IN_STALL {
                self.EX.latch_in_RDest = self.ID.latch_out_RDest;
                self.EX.latch_in_new_pc = self.ID.latch_out_new_pc;
                self.EX.latch_in_A = self.ID.latch_out_A;
                self.EX.latch_in_B = self.ID.latch_out_B;
                self.EX.latch_in_I = self.ID.latch_out_I;
                self.EX.latch_in_instruction_ID = self.ID.latch_out_instruction_ID;
                self.EX.control_in_EXOP = self.ID.control_out_EXOP;
            } else {
                self.EX.latch_in_instruction_ID = self.EX.latch_out_instruction_ID;
            }
            //propagate EX->MEM : only if !EX_IS_STALL
            //NOTE: Some propagation code is contained in the stall check for EX, see above
            if !EX_IN_STALL {
                self.MEM.latch_in_is_privileged = self.EX.latch_out_is_privileged;
                self.MEM.latch_in_instruction_ID = self.EX.latch_out_instruction_ID;
                self.MEM.latch_in_RDest = self.EX.latch_out_RDest;
                self.MEM.latch_in_EX_OUT = self.EX.latch_out_EX_OUT;
            } else {
                //self.MEM.latch_in_instruction_ID = 0;
                self.MEM.latch_in_RDest = None;
                self.MEM.latch_in_EX_OUT = NoOutput;
            }

            //propagate EX->IF (for jumps)
            self.IF.latch_in_is_privileged = self.EX.latch_out_is_privileged;
            self.IF.latch_in_cond = self.EX.latch_out_cond;

            //if there was a jump, reset the propagation of previous stages
            if self.EX.latch_out_cond {
                if self.verbose {
                    println!("[CORE]: Jump enforced, cleared EX and ID stages");
                }

                //Clear signals IF->ID
                self.ID.latch_in_new_pc = 0;
                self.ID.latch_in_IR = (0, 0);

                //Clear signals ID->EX
                self.EX.control_in_EXOP = (InstructionType::Special, 0, 0);
                self.EX.latch_in_A = None;
                self.EX.latch_in_B = None;
                self.EX.latch_in_I = None;
                self.EX.latch_in_RDest = None;

                self.ID.regs.delist_owner(self.EX.latch_in_instruction_ID);
                self.EX.latch_in_instruction_ID = 0;
            }

            //propagate MEM->WB
            self.WB.latch_in_WB_contents = self.MEM.latch_out_WB;
            self.WB.latch_in_RDest = self.MEM.latch_out_RDest;
            self.WB.latch_in_instruction_ID = self.MEM.latch_out_instruction_ID;

            //propagate WB->ID
            self.ID.latch_in_WB_contents = self.WB.latch_out_WB_contents;
            self.ID.latch_in_RDest = self.WB.latch_out_RDest;
            self.ID.latch_in_instruction_ID = self.WB.latch_out_instruction_ID;

            //Was there a stall?
            if EX_IN_STALL && self.verbose {
                println!("[CORE]: Stall\n\tCleared MEM latch_in\n\tStalled IF\n\tStalled ID");
            }
            stat.cycle_incr();
            // if a jump was not enforced and we are not stalling, we can
            // assume an instruction has completed
            if !EX_IN_STALL && !self.EX.latch_out_cond {
                stat.instr_incr();
            }
        }

        if self.verbose {
            println!("[CORE]: Finished execution in T={} s\n        CPI of {}. Executed {} instructions in {} cycles.", stat.exec_total_time().as_secs_f64(), stat.CPI(), stat.instruction_count, stat.cycle_count);
        }

        Ok(())
    }

    fn load_RELF(&mut self, path: &str) -> Result<(), HeaderError> {
        match self.held_mem.load_RELF(path) {
            Ok(pc) => self.IF.set_pc(pc),
            Err(eobj) => return Err(eobj), //propagate
        }

        Ok(())
    }

    fn load_bin(&mut self, path: &str, entry: u32) -> Result<(), Error> {
        self.IF.set_pc(entry);
        self.held_mem.load_bin(path)
    }
}

impl Privileged for Core {
    fn set_privilege(&mut self, privilege: bool) {
        if self.verbose {
            println!("[CORE]: External call to set_privilege: {}", privilege)
        }
        self.IF.latch_in_is_privileged = privilege;
        self.EX.latch_out_is_privileged = privilege;
        self.MEM.latch_in_is_privileged = privilege;
    }
}

#[test]
fn basic() {
    trait AccessControlled: Runnable + Privileged {}
    impl AccessControlled for Core {}
    let mut c: Box<dyn AccessControlled> = Box::new(Core::new(true));
    match c.load_RELF("testbins/test_pipelined_simple.relf") {
        Err(e) => {
            panic!("{}", e)
        }
        Ok(_) => {}
    };
    //avoid depending on a working irqh
    c.set_privilege(true);
    c.run().unwrap();
}

#[test]
fn irqh() {
    let mut c: Core = Core::new(true);
    match c.load_RELF("testbins/test_irqh_pipelined_simple.relf") {
        Err(e) => {
            panic!("{}", e)
        }
        Ok(_) => {}
    };
    c.run().unwrap();
}

#[test]
fn backwards_jump() {
    let mut c: Core = Core::new(true);

    let start = 0x00000010;
    let hlt = [0x42, 0x00, 0x00, 0x10]; //hlt
    let backj = [0x08, 0x00, 0x00, 0x04]; // jmp -1

    c.held_mem.set_privileged(true);
    c.held_mem.store(start, 4, &hlt).unwrap();
    c.held_mem.store(start + 4, 4, &backj).unwrap();
    c.IF.set_pc(start as u32);
    c.held_mem.set_privileged(false);

    c.set_privilege(true);
    c.run().unwrap();
}

/* WIP

#[test]
fn long_compute() {
    let mut c: Core = Core::new(true);

    match c.load_RELF("testbins/perf_test_newcompile.relf") {
        Err(e) => {
            panic!("{}", e)
        }
        Ok(_) => {}
    };

    c.run().unwrap();
}

 */

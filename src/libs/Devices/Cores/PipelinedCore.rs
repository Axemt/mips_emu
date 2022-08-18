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

use crate::libs::Devices::Cores::CoreTraits;
use crate::libs::Devices::Registers::{Available, Registers, SuccessfulOwn};
use crate::libs::Pipeline::Pipelined::Pipelined;
use crate::libs::Pipeline::PipelinedWithHeldMemory::PipelinedWithHeldMemory;
use crate::libs::Pipeline::Stages::Execution::EX_OUT::AwaitingLock;
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
    held_mem: Memory,
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
        regs.write_and_unlock(RegNames::SP as u32, stackbase, 0);
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

impl CoreTraits::Runnable for Core {
    fn run(&mut self) -> Result<(), ExecutionError> {
        loop {
            if self.verbose {
                println!("--------------------");
            }

            if self.EX.control_out_termination {
                if self.verbose {
                    println!("[CORE]: Termination Flag Set");
                }
                break;
                //when breaking, ensure instructions in MEM and WB have completed
            }

            //TODO: Interrupt stuff

            //TICK STEP:
            self.IF.tick_with_mem(&mut self.held_mem)?;
            self.ID.tick()?;
            self.EX.tick()?;
            self.MEM.tick_with_mem(&mut self.held_mem)?;
            self.WB.tick()?;

            //PROPAGATION STEP

            //propagate: IF->ID
            self.ID.latch_in_new_pc = self.IF.latch_out_new_pc;
            self.ID.latch_in_IR = self.IF.latch_out_IR;

            //propagate: WB->ID
            self.ID.latch_in_RDest = self.WB.latch_out_RDest;
            self.ID.latch_in_WB_contents = self.WB.latch_out_WB_contents;

            //propagate: ID->EX

            //Are we stalling in EX?
            match self.EX.latch_out_EX_OUT {
                AwaitingLock(A_lock, B_lock) => {
                    if self.verbose {
                        println!("[CORE]: Stalling EX@{}", self.EX.latch_in_instruction_ID)
                    }
                    // The stage is stalling, do not propagate
                    self.MEM.latch_in_RDest = Some(Ok(SuccessfulOwn { register_number: 0 }));
                    self.MEM.latch_in_EX_OUT = EX_OUT::NoOutput;
                }
                _ => {
                    //propagate: EX->MEM
                    self.MEM.latch_in_RDest = self.EX.latch_out_RDest;
                    self.MEM.latch_in_EX_OUT = self.EX.latch_out_EX_OUT;
                }
            }

            self.EX.latch_in_RDest = self.ID.latch_out_RDest;
            self.EX.latch_in_new_pc = self.ID.latch_out_new_pc;
            self.EX.latch_in_A = self.ID.latch_out_A;
            self.EX.latch_in_B = self.ID.latch_out_B;
            self.EX.latch_in_I = self.ID.latch_out_I;
            self.EX.latch_in_instruction_ID = self.ID.latch_out_instruction_ID;
            self.EX.control_in_EXOP = self.ID.control_out_EXOP;

            //propagate EX->MEM
            //NOTE: Some propagation code is contained in the stall check for EX, see above
            self.MEM.is_privileged = self.EX.is_privileged;
            self.MEM.latch_in_instruction_ID = self.EX.latch_out_instruction_ID;

            //propagate MEM->WB
            self.WB.latch_in_WB = self.MEM.latch_out_WB;
            self.WB.latch_in_RDest = self.MEM.latch_out_RDest;
            self.WB.latch_in_instruction_ID = self.MEM.latch_out_instruction_ID;

            //propagate WB->ID
            self.ID.latch_in_WB_contents = self.WB.latch_out_WB_contents;
            self.ID.latch_in_RDest = self.WB.latch_out_RDest;
            self.ID.latch_in_instruction_ID = self.WB.latch_out_instruction_ID;

            //CONTROL : ReleaseBcast

            // if awaiting a lock on A and/or B, they are held in EX.latch_in_[A,B]
            // if awaiting a lock on RDest, it can be freed at any stage; entering EX, MEM or WB
            match self.ID.control_out_reg_release_bcast {
                ReleaseBcast::NoRelease => {}
                ReleaseBcast::Freed(id, regno) => {
                    if self.verbose {
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
                }
                ReleaseBcast::FreedWithContent(id, regno, v) => {
                    if self.verbose {
                        println!("[CORE]: Broadcasting Free of register {regno} owned by {id}")
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
                }
                ReleaseBcast::FreedWithContentDouble(id, hi, lo) => {}
            }
        }

        Ok(())
    }

    fn load_RELF(&mut self, path: &str) -> Result<(), HeaderError> {
        match self.held_mem.load_RELF(path) {
            Ok(pc) => self.IF.pc = pc,
            Err(eobj) => return Err(eobj), //propagate
        }

        Ok(())
    }

    fn load_bin(&mut self, path: &str, entry: u32) -> Result<(), std::io::Error> {
        self.IF.pc = entry;
        self.held_mem.load_bin(path)
    }
}

impl CoreTraits::Privileged for Core {
    fn set_privilege(&mut self, privilege: bool) {
        if self.verbose {
            println!("[CORE]: External call to set_privilege: {}", privilege)
        }
        self.EX.is_privileged = privilege;
        self.MEM.is_privileged = privilege;
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
    c.set_privilege(true); //irqh and jumps not working at the moment, do not rely on syscall 10
    c.run().unwrap();
}

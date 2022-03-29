use super::Memory::Memory;
use super::Definitions::Utils::{Byte, Half, Word};
use super::Definitions::{Utils, Stats};
use super::Definitions::Arch;
use super::Definitions::Arch::{OP, RegNames};

use super::Definitions::Errors::{ExecutionError, HeaderError};

use super::Devices::{MemoryMapped,Console,Keyboard,Interruptor};

use crate::to_signed;
use crate::to_signed_cond;

use std::time::Duration;
use std::sync::mpsc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;


pub struct Core {

    reg: [Word ; 32],
    HI: Word,
    LO: Word,
    mem: Memory,
    flags: Word,
    PC: u32,
    irq_handler_addr: u32,
    EPC: u32,
    IntEnableOnNext: bool,
    verbose: bool,
    interrupt_ch: mpsc::Receiver<u32>,
    interrupt_ch_open: Arc<AtomicBool>
}



impl Core {

    pub fn new(v: bool) -> Core {
        //everything is supposed to be ok in this constructor, no need to use Result
    
        let mut mem = Memory::new(v);
        let mut reg = [0; 32];

        //init default irq_handler
        mem.set_privileged(true);
        let DEFAULT_irq = Arch::DEFAULT_IRQH;
        let irq_addr: u32 = 0x0;
    
        if v { println!("[CORE]: Setting up default IRQH with address 0x{:08x}",irq_addr) }
    
        let stackbase = DEFAULT_irq.len() as u32 + 8;
        mem.store(irq_addr as usize, DEFAULT_irq.len(),&DEFAULT_irq).unwrap();
        mem.protect(irq_addr,stackbase - 4);

        if v { println!("[CORE]: Setting up stack from 0x{:08x} to 0x{:08x}",stackbase, stackbase + Arch::STACKSIZE); }

        mem.protect(stackbase, stackbase + Arch::STACKSIZE);
        reg[RegNames::SP] = stackbase;
        mem.set_privileged(false);
    
        //add basic mapped devices
        let console  = Box::new(Console::new() );
        let keyboard = Box::new(Keyboard::new() );
        mem.map_device( console.range_lower,console.range_upper, console  );
        mem.map_device( keyboard.range_lower, keyboard.range_upper, keyboard);
    
        let (send, recv) = mpsc::channel();
    
        let mut core = Core {
            reg: reg,
            HI: 0,
            LO: 0,
            mem: mem,
            flags: 0,
            PC: 0,
            irq_handler_addr: irq_addr,
            EPC: 0,
            IntEnableOnNext: false,
            verbose: v,
            interrupt_ch: recv,
            interrupt_ch_open: Arc::new(AtomicBool::new(true))
        };
        core.set_flag(true, Arch::IENABLE_FLAG);

        Interruptor::new_default("Clock", Duration::new(1, 0), &send, core.interrupt_ch_open.clone(), v);
        
        if v { println!("[CORE]: Created successfully!\n"); }
        
        core
    }

    /**
     * Loads a RELF executable into memory and sets PC
     *
     * ARGS:
     *
     *  path: Path to executable
    */
    pub fn load_RELF(&mut self, path: &str) -> Result<(), HeaderError>{

        match self.mem.load_RELF(path) {
            Ok(pc) => self.PC = pc,
            Err(eobj) => return Err(eobj) //propagate
        }

        Ok(())
    }

    /**
     * Loads a raw binary into memory and sets PC
     *
     * Note that this starts writing from address 0x00000000
     * and ignores reserved sections of memory
     *
     * ARGS:
     *
     *  path: Path to binary file
     *
     *  entry: PC to start execution
    */
    pub fn load_bin(&mut self, path: &str, entry: u32) -> Result<(), std::io::Error> {

        self.PC = entry;
        self.mem.load_bin(path)

    }

    /**
     * Interrupts current execution and jumps to irqH
     *
     */

    pub fn interrupt(&mut self) {
        self.set_flag(true, Arch::MODE_FLAG); //enter privileged mode
        self.mem.set_privileged(true);
        self.set_flag(false, Arch::IENABLE_FLAG); //disable interrupts
        self.EPC = self.PC;
        self.PC = u32::saturating_sub(self.irq_handler_addr, 4);
    }

    /**
     *  Activates or deactivates a flag on the processor
     *
     *  ARGS:
     *
     *  set: The value to set the flag to
     *
     *  flag: The flag to modify
     */
    pub fn set_flag(&mut self,set: bool, flag: u32) {


        if self.verbose { println!("[CORE]: Setting flag {flag} to {set}"); }

        if set {

            self.flags |= flag;

        } else {

            self.flags &= !flag;
        }

    }

    /**
     *  Adds the range proct_low .. proct_high to the set of protected address ranges
     *
     *  ARGS:
     *
     *  proct_low: Lowest address of the reserved range
     *
     *  proct_high: Highest address of the reserved range
     *
     */
    #[cfg(not(tarpaulin_include))]
    #[allow(dead_code)]
    pub fn protect_mem(&mut self, proct_low: u32, proct_high: u32) {
        self.mem.protect(proct_low,proct_high);
    }

    /**
     * Sets the IrqH PC for exceptions
     *
     * ARGS:
     *
     * irqpc: The address to jump to on interrupt
     */
    #[cfg(not(tarpaulin_include))]
    #[allow(dead_code)]
    pub fn set_IrqHPC(&mut self, irq_pc: u32) {
        self.irq_handler_addr = irq_pc
    }

    /**
     * Starts running code at PC.
     *
     * Note: this is an infinite loop, please end your code segments via a syscall 10
    */
    pub fn run(&mut self) -> Result<(), ExecutionError> {

        let mut stat = Stats::new();

        let mut iter_flag = false;

        loop {
            if self.verbose { println!("------------------"); }

            self.run_handoff(self.PC)?;
            stat.cycle_incr();
            stat.instr_incr();

            //increment pc, set $0 to constant
            self.PC += 4;
            self.reg[RegNames::ZERO] = 0;

            // end of instruction routines

            //check if FIN_FLAG is set
            if (self.flags & Arch::FIN_FLAG) != 0 {
                if self.verbose { println!("------------------\n[CORE]: FIN_FLAG set; Flags={:08x}",self.flags) }
                stat.mark_finished();
                break;
            }

            // flag set if the previous instruction was RFE. We ensure progress by allowing
            // at least one instruction executes before the interrupt handler fires again.
            // There's probably a better way to do this

            if iter_flag {
                iter_flag = false;
                self.set_flag(true, Arch::IENABLE_FLAG);
            }

            //else, check if INTERR_FLAG is set in channel only if not privileged
            if (self.flags & Arch::IENABLE_FLAG) != 0 && (self.flags & Arch::MODE_FLAG) == 0 {
                match self.interrupt_ch.try_recv() {
                    Ok(_) => { self.set_flag(true, Arch::INTERR_FLAG); },
                    Err(_) => {},
                }

                //interrupt flag set in channel
                if (self.flags & Arch::INTERR_FLAG) != 0 {
                    if self.verbose { println!("[CORE]: INTERR_FLAG set; Flags={:08x}",self.flags) }
                    self.set_flag(true, Arch::INTERR_FLAG);
                    // This is a horrible hack
                    // This is only needed here because the interrupt happens *after* pc has been incremented, instead of in every interrupt(like syscalls)
                    self.PC -= 4;
                    self.interrupt();
                }
            }

            if self.IntEnableOnNext {
                self.IntEnableOnNext = false;
                iter_flag = true;
            }

        }

        if self.verbose {
            println!("[CORE]: Finished execution in T={} s\n        CPI of {}. Executed {} instructions in {} cycles.",stat.exec_total_time().as_secs_f64(),stat.CPI(),stat.instr_count, stat.cycl_count);
        }

        Ok(())
    }

    #[inline(always)]
    fn run_handoff(&mut self, PC: u32) -> Result<(), ExecutionError> {

        //avoid weird tuples in vec::align_to, we know it'll always be aligned
        let code: Word = Utils::from_word(self.mem.load(PC,4)?);


        if self.verbose {
            println!("[CORE]: Code: 0x{:08x?} at PC=0x{:08x}",code,PC);
        }

        let maskOP = (code & 0xfc000000) >> 26;
        if maskOP == 0 {
            //is an R-type instruction
            self.handoff_R(code)?;

        } else if ! (maskOP == 0b000010 || maskOP == 0b000011 || maskOP == 0b011010) {
            // is an I-type instruction
            self.handoff_I(code)?;
        } else {
            // is a J-type instruction
            self.handoff_J(code)?;
        }

        Ok(())
    }


    fn handoff_R(&mut self,code: Word) -> Result<(), ExecutionError> {

        if code == OP::NOP {
            if self.verbose { println!("\tR-type: NOP"); }
            return Ok(());
        }

        let rs   = self.reg[((code & 0b00000011111000000000000000000000) >> 21) as usize];
        let rt   = self.reg[((code & 0b00000000000111110000000000000000) >> 16) as usize];
        let rd   = ((code & 0b00000000000000001111100000000000) >> 11) as usize;
        let sham = (code & 0b00000000000000000000011111000000) >> 6;
        let func = code & 0b00000000000000000000000000111111;

        let rt_sign_positive = rt & 0x80000000 == 0;
        let rs_sign_positive = rs & 0x80000000 == 0;

        if self.verbose {
            println!("\tR-type: rs={} rt={} rd={} sham={} func={:02x}; code =0x{:08x?}",rs,rt,rd,sham,func,code);
        }

        //non-zero value for flag check after
        let mut res = 1;
        match func {
            OP::R::ADD   => {res = if rt_sign_positive { rs.overflowing_add(rt).0 } else { rs.overflowing_sub(rt).0 }; self.reg[rd] = res},   //add
            OP::R::ADDU  => {res = rs.overflowing_add(rt).0; self.reg[rd] = res;},   //addu
            OP::R::AND   => {res = rs & rt; self.reg[rd] = res;},   //and
            OP::R::NOR   => {res = !(rs | rt); self.reg[rd] = res;},//nor
            OP::R::OR    => {res = rs | rt; self.reg[rd] = res;},   //or
            OP::R::SUB   => {res = rs.overflowing_sub(rt).0; self.reg[rd] = res;} ,  //sub
            OP::R::SUBU  => {res = rs.overflowing_sub(rt).0; self.reg[rd] = res;},   //subu
            OP::R::XOR   => {res = rs ^ rt; self.reg[rd] = res;},   //xor
            OP::R::SLT   => { if to_signed_cond!(rs, rs_sign_positive, u32) < to_signed_cond!(rt, rt_sign_positive, u32) {self.reg[rd] = 1} else {self.reg[rd] = 0} },  //slt FLAGS NOT IMPLEMENTED!
            OP::R::SLTU  => { if rs < rt {res = 1} else {res = 0}; self.reg[rd] = res;}, //sltu
            OP::R::DIV   => {self.LO = rs / rt; self.HI = rs % rt;},          //div
            OP::R::DIVU  => {self.LO = rs.saturating_div(rt); self.HI = rs % rt;}, //divu
            OP::R::MULT  => { (self.HI, self.LO) = (to_signed_cond!(rs, rs_sign_positive, u32)).widening_mul(to_signed_cond!(rt, rt_sign_positive, u32)); },//mult
            OP::R::MULTU => { (self.HI, self.LO) = rs.widening_mul(rt); },//multu
            OP::R::SLL   => {res = rt << sham; self.reg[rd] = res;},//sll
            OP::R::SRA   => {res = (rt as i32 >> sham as i32) as u32; self.reg[rd] = res;},//sra ; for rust to do shift aritmetic, use signed types
            OP::R::SRAV  => {res = (rt as i32 >> rs as i32) as u32; self.reg[rd] = res;},   //srav; for rust to do shift aritmetic, use signed types
            OP::R::SRLV  => {res = rt >> rs; self.reg[rd] = res;},//srlv
            OP::R::JARL  => {self.reg[RegNames::RA] = self.PC; self.PC = if rs != 0 {rs-4} else {rs};},//jalr
            OP::R::JR    => {self.PC = if rs != 0 {rs-4} else {rs}},//jr
            OP::R::MFHI  => {self.reg[rd] = self.HI;},//mfhi
            OP::R::MFLO  => {self.reg[rd] = self.LO;},//mflo
            OP::R::MTHI  => {self.HI = rs;},//mthi
            OP::R::MTLO  => {self.LO = rs;},//mtlo


            _ => { return Err(ExecutionError::UnrecognizedOPError(format!("Unrecognized R type func {:x}",func))) ;}

        }

        self.set_flag(res == 0, Arch::Z_FLAG);
        self.set_flag((res as i32) < 0, Arch::S_FLAG);

        Ok(())

    }


    fn handoff_I(&mut self, code: Word) -> Result<(), ExecutionError> {

        //special instruction: RFE
        if code == OP::RFE {

            let privileged = (self.flags & Arch::MODE_FLAG) != 0;


            if self.verbose { println!("\tRFE:EPC={:08x}; privilege status {}. Flags {:08x}",self.EPC, privileged,self.flags); }

            //panic if we are not privileged
            if  !privileged { 
                return Err(ExecutionError::PrivilegeError(String::from("RFE")));
                //panic!("Tried to use privileged instruction 0x{:08x} but the mode bitflag was not set to 1; Flags=0x{:08x}",code, self.flags); 
            }

            //restore PC
            self.PC = self.EPC; 
            //disable privileged
            self.set_flag(false,Arch::MODE_FLAG);
            self.IntEnableOnNext = true;
            self.set_flag(false, Arch::INTERR_FLAG);

            if self.verbose { println!("[CORE]: Changed privilege mode to false") }

            self.mem.set_privileged(false);

            return Ok(());

        }

        //special instruction: hlt
        if code == OP::HLT {

            let privileged = (self.flags & Arch::MODE_FLAG) != 0;

            if self.verbose { println!("\tHLT: privilege status: {};",privileged ); }

            //panic if we are not privileged
            if  !privileged { 
                return Err(ExecutionError::PrivilegeError(String::from("HLT")));
                //panic!("Tried to use privileged instruction 0x{:08x} but the mode bitflag was not set to 1; Flags=0x{:08x}",code, self.flags); 
            }

            if self.verbose { println!("[CORE]: Sending interrupt channel close signal"); }
            self.interrupt_ch_open.swap(false, Ordering::Relaxed);

            //set fin flag, disable privileged
            self.set_flag(false, Arch::MODE_FLAG);
            self.set_flag(true, Arch::FIN_FLAG);
            self.mem.set_privileged(false);
            //"await" interrupt channel termination
            while !self.interrupt_ch.recv_timeout(Duration::new(0, 1)).is_err() {}

            return Ok(());

        }


        let func = (code & 0b11111100000000000000000000000000) >> 26;
        let rs   = self.reg[((code & 0b00000011111000000000000000000000) >> 21) as usize];
        let rt   = ((code & 0b00000000000111110000000000000000) >> 16) as usize;
        let imm  = code & 0b00000000000000001111111111111111;

        let imm_sign_positive  = (code & 0b00000000000000001000000000000000) == 0;

        if self.verbose {
            println!("\tI-type: func={:02x} rs={} rt={} imm={}{} ; code =0x{:08x?}",func,rs,rt,if imm_sign_positive {"+"} else {"-"} ,to_signed!(imm,u16),code);
        }

        match func {
            OP::I::ADDI  => {
                //using signed, if number is negative subtract
                self.reg[rt] = if imm_sign_positive { rs.overflowing_add(imm).0 } else { rs.overflowing_sub(imm).0 };
            },//addi
            OP::I::ADDIU => {self.reg[rt] = rs.overflowing_add(imm).0;}//addiu
            OP::I::ANDI  => {self.reg[rt] = rs & imm;}//andi
            OP::I::ORI   => {self.reg[rt] = rs | imm;}//ori
            OP::I::XORI  => {self.reg[rt] = rs ^ imm;}//xori
            OP::I::SLTI  => {if rs < imm { self.reg[rt] = 1;} else { self.reg[rt] = 0;} } //slti
            OP::I::SLTIU => {if rs < imm { self.reg[rt] = 1;} else { self.reg[rt] = 0;} }//sltiu
            OP::I::LHI   => {println!("lhi");}//lhi
            OP::I::LLO   => {println!("llo");}//llo
            OP::I::BEQ   => { if rs == self.reg[rt] { if imm_sign_positive { self.PC = self.PC.overflowing_add(imm << 2).0;} else { self.PC = self.PC.overflowing_sub(to_signed!(imm<<2, u16)).0 }}; }//beq
            OP::I::BNE   => { if rs != self.reg[rt] { if imm_sign_positive { self.PC = self.PC.overflowing_add(imm << 2).0;} else { self.PC = self.PC.overflowing_sub(to_signed!(imm<<2, u16)).0 }}; }//bne
            OP::I::BGTZ  => { if rs > 0             { if imm_sign_positive { self.PC = self.PC.overflowing_add(imm << 2).0;} else { self.PC = self.PC.overflowing_sub(to_signed!(imm<<2, u16)).0 }}; }//bgtz
            OP::I::BLEZ  => { if rs <= self.reg[rt] { if imm_sign_positive { self.PC = self.PC.overflowing_add(imm << 2).0;} else { self.PC = self.PC.overflowing_sub(to_signed!(imm<<2, u16)).0 }}; }//blez
            OP::I::LB    => {self.reg[rt] = Utils::from_byte(self.mem.load(rs+imm, 1)? );}//lb
            OP::I::LBU   => {self.reg[rt] = Utils::from_byte(self.mem.load(rs+imm, 1)? );}//lbu
            OP::I::LH    => {self.reg[rt] = Utils::from_half(self.mem.load(rs+imm, 2)? );}//lh
            OP::I::LHU   => {self.reg[rt] = Utils::from_half(self.mem.load(rs+imm, 2)? );}//lhu
            OP::I::LW    => {self.reg[rt] = Utils::from_word(self.mem.load(rs+imm, 4)? );}//lw
            OP::I::SB    => {
                let b = self.reg[rt];
                let v = vec![b as u8;1];

                self.mem.store( (rs+imm) as usize , 1, &v)?;
            }//sb
            OP::I::SH => {
                let b = self.reg[rt];
                let v = vec![(b >> 8) as u8, (b & 0x00ff) as u8];

                self.mem.store( (rs+imm) as usize, 2, &v)?;
            }//sh
            OP::I::SW => {
                let b = self.reg[rt];
                let v = vec![(b & 0xff000000 >> 24) as u8, (b & 0x00ff0000 >> 16) as u8,(b & 0x0000ff00 >> 8) as u8, (b & 0x000000ff) as u8];

                self.mem.store( (rs +imm) as usize, 4, &v)?;
            }//sw


            _ => { return Err(ExecutionError::UnrecognizedOPError(format!("Unrecognized I type func {:x}",func))) }


        }

        Ok(())

    }


    fn handoff_J(&mut self,code: Word) -> Result<(), ExecutionError> {

        //special instruction, syscall
        if code == OP::SYSCALL {

            if self.verbose { println!("\tSyscall; v0={}, v1={}\n[CORE]: Changed privilege mode to true", self.reg[RegNames::V0], self.reg[RegNames::V1]); }

            //save current pc, jump to IrqH, set privileged flag
            self.interrupt();
            return Ok(());
        }

        let func          = (code & 0xfc000000) >> 26;
        let jump_target   = (code & !0xfc000000) << 2 ;

        if self.verbose { println!("\tJ type: func=0x{:02x} jump_target=0x{:08x}",func,jump_target); }

        match func {
            OP::J::J   => {self.PC = if jump_target != 0 {jump_target-4} else {jump_target};}
            OP::J::JAL => {self.reg[RegNames::RA] = self.PC; self.PC = if jump_target != 0 {jump_target-4} else {jump_target};}

            _ => { return Err(ExecutionError::UnrecognizedOPError(format!("Unrecognized J type func {:02x}",func))); }
        }

        Ok(())
    }


}

/**
 *  TESTS
 */


#[test]
fn basic() {

    let mut c: Core = Core::new(true);

    match c.load_RELF("src/libs/testbins/testingLS.s.relf") {
        Err(e) => { panic!("{}", e) }
        Ok(_) => {}
    }
    c.run().unwrap();
}

#[test]
fn backwards_jumps() {
    let mut c: Core = Core::new(true);

    let start = 0x00000010;
    let hlt = [0x42, 0x00, 0x00, 0x10]; //hlt
    let backj = [0x08, 0x00, 0x00, 0x04]; // jmp -1

    c.mem.set_privileged(true);
    c.set_flag(true, Arch::MODE_FLAG);

    c.mem.store(start, 4, &hlt).unwrap();
    c.mem.store(start + 4, 4, &backj).unwrap();
    c.PC = start as u32;

    c.run().unwrap();
}

#[test]
fn long_compute() {

    let mut c: Core = Core::new(true);

    match c.load_RELF("src/libs/testbins/perf_test.s.relf") {
        Err(e) => { panic!("{}",e) }
        Ok(_) => {}
    };

    c.run().unwrap();
}


#[test]
#[should_panic]
fn unprivileged_rfe() {
    let mut c: Core = Core::new(true);

    let code = [0x42, 0x00, 0x00, 0x01]; //rfe
    c.mem.store(0x00fff, 4, &code).unwrap();
    c.PC = 0x00fff;

    c.run().unwrap();
}

#[test]
#[should_panic]
fn unprivileged_hlt() {
    let mut c: Core = Core::new(true);

    let code = [0x42, 0x00, 0x00, 0x10]; //hlt
    c.mem.store(0x00fff, 4, &code).unwrap();
    c.PC = 0x00fff;

    c.run().unwrap();

}

#[test]
fn default_irqH() {
    let mut c: Core = Core::new(true);
 
    c.mem.store(0xff0f8, 4, &[0x20, 0x02, 0x00, 0x0A]).unwrap(); //li $v0, 10
    c.mem.store(0xff0fc, 4, &[0x68,0x00,0x00,0x00]).unwrap(); //syscall
    c.PC = 0xff0f8;

    c.run().unwrap();
}

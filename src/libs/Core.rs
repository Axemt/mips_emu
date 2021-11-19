use super::Memory;
use super::Definitions;
use super::Definitions::{Byte, Half, Word};
use std::panic;
use std::time::Instant;
use std::time::Duration;
use super::Devices::MemoryMapped;
use super::Devices::{Console,Keyboard,Interruptor};
use std::sync::mpsc;


pub struct Core {

    reg: Vec<Word>,
    HI: Word,
    LO: Word,
    mem: Memory::Memory,
    flags: Word,
    PC: u32,
    irq_handler_addr: u32,
    EPC: u32,
    verbose: bool,
    interrupt_ch: mpsc::Receiver<u32>
}


pub fn new(v: bool) -> Core {

    let mut mem = Memory::new(v);
    //init default irq_handler
    mem.set_privileged(true);
    let DEFAULT_irq = Definitions::DEFAULT_IRQH;
    let irq_addr: u32 = 0x0;
    if v { println!("[CORE]: Setting up default IRQH with address {:08x}",irq_addr) }
    mem.store(irq_addr as usize, DEFAULT_irq.len(),&DEFAULT_irq);
    mem.protect(irq_addr,DEFAULT_irq.len() as u32 + 4);
    mem.set_privileged(false);

    //add basic mapped devices
    let console  = Box::new(Console::new() );
    let keyboard = Box::new(Keyboard::new() );
    mem.map_device( console.range_lower,console.range_upper, console  );
    mem.map_device( keyboard.range_lower, keyboard.range_upper, keyboard);

    let (send, recv) = mpsc::channel();
    Interruptor::new_default("Clock", 1, &send, v);

    let c = Core {reg: vec![0;32],HI: 0, LO: 0, mem: mem, flags: 0 ,PC: 0, irq_handler_addr: irq_addr, EPC: 0, verbose: v, interrupt_ch: recv};


    return c;

}


impl Core {

    /**
     * Loads a RELF executable into memory and sets PC
     * 
     * ARGS:
     * 
     *  path: Path to executable 
    */
    pub fn load_RELF(&mut self, path: &str) {

        self.PC = self.mem.load_RELF(path);

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
    pub fn load_bin(&mut self, path: &str, entry: u32) {

        self.PC = entry;
        self.mem.load_bin(path);

    }

    /**
     * Interrupts current execution and jumps to irqH
     * 
     */

    pub fn interrupt(&mut self) {
        self.set_flag(true, Definitions::MODE_FLAG);
        self.mem.set_privileged(true);
        self.EPC = self.PC;
        self.PC = if self.irq_handler_addr != 0 {self.irq_handler_addr-4} else { self.irq_handler_addr };
    }

    pub fn set_flag(&mut self,set: bool, flag: u32) {

        if set { 

            self.flags |= flag;

        } else { 

            //prepare bitmask
            self.flags &= 0xffffffff & !flag;
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
    pub fn set_IrqHPC(&mut self, irqpc: u32) {
        self.irq_handler_addr = irqpc
    }

    /**
     * Starts running code at PC. 
     * 
     * Note: this is an infinite loop, please end your code segments via a syscall
    */
    pub fn run(&mut self) {

        let st = Instant::now();

        loop {
            if self.verbose { println!("------------------"); }

            self.run_handoff(self.PC);

            //increment pc, set reg[0] to constant
            self.PC += 4;
            self.reg[0] = 0;

            //check if INTERR_FLAG is set in channel only if not privileged
            if (self.flags & Definitions::MODE_FLAG) == 1 {
                let received = self.interrupt_ch.try_recv().unwrap();

                //interrupt flag set in channel
                if received == 1 {
                    if self.verbose { println!("[CORE]: INTERR_FLAG set; Flags={:08x}",self.flags) }
                    self.set_flag(true, Definitions::INTERR_FLAG);
                    self.interrupt();
                }
            }

            //check if FIN_FLAG is set
            if (self.flags & Definitions::FIN_FLAG) != 0 {
                if self.verbose { println!("------------------\n[CORE]: FIN_FLAG set; Flags={:08x}",self.flags) }
                break;
            }

        }

        if self.verbose {
            println!("[CORE]: Finished execution in T={} s",st.elapsed().as_secs_f64())
        }

    }

    #[inline(always)]
    fn run_handoff(&mut self, PC: u32) {

        //avoid weird tuples in vec::align_to, we know it'll always be aligned
        let code: Word = Definitions::from_word(self.mem.load(PC,4));


        if self.verbose {
            println!("[CORE]: Code: 0x{:08x?} at PC=0x{:08x}",code,PC);
        }

        let maskOP = (code & 0xfc000000) >> 26;
        if maskOP == 0 {
            //is an R-type instruction
            self.handoff_R(code);

        } else if ! (maskOP == 0b000010 || maskOP == 0b000011 || maskOP == 0b011010) {
            // is an I-type instruction
            self.handoff_I(code);
        } else {
            // is a J-type instruction
            self.handoff_J(code);
        }
    }


    fn handoff_R(&mut self,code: Word) {

        if code == 0x00000000 {
            if self.verbose { println!("\tR-type: NOP"); }
            return;
        }

        let rs   = self.reg[((code & 0b00000011111000000000000000000000) >> 21) as usize];
        let rt   = self.reg[((code & 0b00000000000111110000000000000000) >> 16) as usize];
        let rd   = ((code & 0b00000000000000001111100000000000) >> 11) as usize;
        let sham = (code & 0b00000000000000000000011111000000) >> 6;
        let func = code & 0b00000000000000000000000000111111;

        if self.verbose { 
            println!("\tR-type: rs={} rt={} rd={} sham={} func={:02x}; code =0x{:08x?}",rs,rt,rd,sham,func,code); 
        }

        //TODO implement type-ish system/encoding for unsigned
        //non-zero value for flag check after
        let mut res = 1;
        match func {
            0b100000 => {res = (rs as i32 + rt as i32) as u32; self.reg[rd] = res as u32;},   //add
            0b100001 => {res = rs + rt; self.reg[rd] = res;},   //addu
            0b100100 => {res = rs & rt; self.reg[rd] = res;},   //and
            0b100111 => {res = !(rs | rt); self.reg[rd] = res;},//nor
            0b100101 => {res = rs | rt; self.reg[rd] = res;},   //or
            0b100010 => {res = (rs as i32 - rt as i32) as u32; self.reg[rd] = res;} ,  //sub
            0b100011 => {res = rs - rt; self.reg[rd] = res;},   //subu
            0b100110 => {res = rs ^ rt; self.reg[rd] = res;},   //xor
            0b101010 => { println!("slt"); },  //slt FLAGS NOT IMPLEMENTED!
            0b101001 => { println!("sltu"); }, //sltu
            0b011010 => {self.LO = rs / rt; self.HI = rs % rt;},          //div
            0b011011 => {self.LO = (rs / rt) as u32; self.HI = rs % rt;}, //divu
            0b011000 => {let m = rs * rt;self.HI = m >> 16; self.LO = m << 16 ;},//mult
            0b011001 => {let m = rs * rt;self.HI = m >> 16; self.LO = m << 16 ;},//multu
            0b000000 => {res = rt << sham; self.reg[rd] = res;},//sll
            0b000011 => {res = (rt as i32 >> sham as i32) as u32; self.reg[rd] = res;},//sra ; for rust to do shift aritmetic, use signed types
            0b000111 => {res = (rt as i32 >> rs as i32) as u32; self.reg[rd] = res;},   //srav; for rust to do shift aritmetic, use signed types
            0b000110 => {res = rt >> rs; self.reg[rd] = res;},//srlv
            0b001001 => {self.reg[31] = self.PC; self.PC = if rs != 0 {rs-4} else {rs};},//jalr
            0b001000 => {self.PC = if rs != 0 {rs-4} else {rs}},//jr
            0b010000 => {self.reg[rd] = self.HI;},//mfhi
            0b010010 => {self.reg[rd] = self.LO;},//mflo
            0b010001 => {self.HI = rs;},//mthi
            0b010011 => {self.LO = rs;},//mtlo


            _ => {panic!("Unrecognized R type func {:x}",func);}

        }

        self.set_flag(res == 0, Definitions::Z_FLAG);
        self.set_flag((res as i32) < 0, Definitions::S_FLAG);

    }


    fn handoff_I(&mut self, code: Word) {

        //special instruction: RFE
        if code == 0x42000001 {

            let privileged = (self.flags & Definitions::MODE_FLAG) != 0;


            if self.verbose { println!("\tRFE:EPC={:08x}; privilege status {}. Flags {:08x}",self.EPC, privileged,self.flags); }

            //panic if we are not privileged
            if  !privileged { panic!("Tried to use privileged instruction 0x{:08x} but the mode bitflag was not set to 1; Flags=0x{:08x}",code, self.flags); }
            
            //restore PC
            self.PC = self.EPC;
            //disable privileged
            self.set_flag(false,Definitions::MODE_FLAG);

            if self.verbose { println!("[CORE]: Changed privilege mode to false") }

            self.mem.set_privileged(false);

            return;

        }

        //special instruction: hlt
        if code == 0x42000010 {

            let privileged = (self.flags & Definitions::MODE_FLAG) != 0;

            if self.verbose { println!("\tHLT: privilege status: {};",privileged ); }

            //panic if we are not privileged
            if  !privileged { panic!("Tried to use privileged instruction 0x{:08x} but the mode bitflag was not set to 1; Flags=0x{:08x}",code, self.flags); }
            
            //set fin flag, disable privileged
            self.set_flag(false, Definitions::MODE_FLAG);
            self.mem.set_privileged(false);
            
            self.set_flag(true, Definitions::FIN_FLAG);

            return;

        }


        let func = (code & 0b11111100000000000000000000000000) >> 26; 
        let rs   = self.reg[((code & 0b00000011111000000000000000000000) >> 21) as usize];
        let rt   = ((code & 0b00000000000111110000000000000000) >> 16) as usize;
        let imm  = code & 0b00000000000000001111111111111111;
        
        if self.verbose { 
            println!("\tI-type: func={:02x} rs={} rt={} imm={} ; code =0x{:08x?}",func,rs,rt,imm,code); 
        }

        match func {
            0b001000 => {self.reg[rt] = (rs as i32 + imm as i32) as u32;},//addi
            0b001001 => {self.reg[rt] = rs + imm;},//addiu
            0b001100 => {self.reg[rt] = rs & imm;},//andi
            0b001101 => {self.reg[rt] = rs | imm;},//ori
            0b001110 => {self.reg[rt] = rs ^ imm;},//xori
            0b001010 => {if (rs as i32) < (imm as i32) { self.reg[rt] = 1;} else { self.reg[rt] = 0;} }, //slti
            0b001011 => {if rs < imm { self.reg[rt] = 1;} else { self.reg[rt] = 0;} },//sltiu
            0b011001 => {println!("lhi");},//lhi
            0b011000 => {println!("llo");},//llo
            0b000100 => { if rs == self.reg[rt] { self.PC += (imm as u32) << 2;}; },//beq
            0b000101 => { if rs != self.reg[rt] { self.PC += (imm as u32) << 2;}; },//bne
            0b000111 => { if rs > 0             { self.PC += (imm as u32) << 2;}; },//bgtz
            0b000110 => { if rs <= self.reg[rt] { self.PC += (imm as u32) << 2;}; },//blez
            0b100000 => {self.reg[rt] = Definitions::from_byte(self.mem.load(rs+imm, 1) );},//lb
            0b100100 => {self.reg[rt] = Definitions::from_byte(self.mem.load(rs+imm, 1) );},//lbu
            0b100001 => {self.reg[rt] = Definitions::from_half(self.mem.load(rs+imm, 2) );},//lh
            0b100101 => {self.reg[rt] = Definitions::from_half(self.mem.load(rs+imm, 2) );},//lhu
            0b100011 => {self.reg[rt] = Definitions::from_word(self.mem.load(rs+imm, 4) );}//lw
            0b101000 => {
                let b = self.reg[rt];
                let v = vec![b as u8;1];

                self.mem.store( (rs+imm) as usize , 1, &v);
            },//sb
            0b101001 => {
                let b = self.reg[rt];
                let v = vec![(b >> 8) as u8, (b & 0x00ff) as u8];

                self.mem.store( (rs+imm) as usize, 2, &v);
            },//sh
            0b101011 => {
                let b = self.reg[rt];
                let v = vec![(b & 0xff000000 >> 24) as u8, (b & 0x00ff0000 >> 16) as u8,(b & 0x0000ff00 >> 8) as u8, (b & 0x000000ff) as u8];

                self.mem.store( (rs +imm) as usize, 4, &v);
            },//sw


            _ => { panic!("Unrecognized I type func {:x}",func) }



        }

    }


    fn handoff_J(&mut self,code: Word) {

        //special instruction, syscall
        if code == 0x68000000 {

            if self.verbose { println!("\tSyscall; v0={}, v1={}\n[CORE]: Changed privilege mode to true", self.reg[2], self.reg[3]); }

            //save current pc, jump to IrqH, set privileged flag
            self.interrupt();
            return;
        }

        let func          = (code & 0xfc000000) >> 26;
        let jump_target   = (code & !0xfc000000) << 2 ;

        if self.verbose { println!("\tJ type: func=0x{:02x} jump_target=0x{:08x}",func,jump_target); }

        match func {
            0b000010 => {self.PC = if jump_target != 0 {jump_target-4} else {jump_target};}
            0b000011 => {self.reg[31] = self.PC; self.PC = if jump_target != 0 {jump_target-4} else {jump_target};}

            _ => { panic!("Unrecognized J type func {:02x}",func); }
        }


    }


}

/**
 *  TESTS
 */


#[test]
fn basic() {

    let mut c: Core = new(true);

    c.load_RELF("src/libs/testbins/testingLS.s.relf");
    c.run();

    
}


#[test]
#[should_panic]
fn unprivileged_rfe() {
    let mut c: Core = new(true);

    let code = [0x42, 0x00, 0x00, 0x01]; //rfe
    c.mem.store(0x00fff, 4, &code);
    c.PC = 0x00fff;

    c.run();
}

#[test]
#[should_panic]
fn unprivileged_hlt() {
    let mut c: Core = new(true);

    let code = [0x42, 0x00, 0x00, 0x10]; //hlt
    c.mem.store(0x00fff, 4, &code);
    c.PC = 0x00fff;

    c.run();

}

#[test]
fn default_irqH() {
    let mut c: Core = new(true);

    c.mem.store(0x00f8, 4, &[0x20, 0x02, 0x00, 0x0A]); //li $v0, 10
    c.mem.store(0x00fc, 4, &[0x68,0x00,0x00,0x00]); //syscall
    c.PC = 0x00f8;

    c.run();
}

use super::Memory;
use super::Definitions;
use super::Definitions::{Byte, Half, Word};
use std::panic;
use std::time::Instant;

pub struct Core {

    reg: Vec<u32>,
    HI: u32,
    LO: u32,
    mem: Memory::Memory,
    flags: u32,
    PC: u32,
    IrqH_addr: u32,
    EPC: u32,
    verbose: bool
}


pub fn new(v: bool) -> Core {

    let mut c = Core {reg: vec![0;32],HI: 0, LO: 0, mem: Memory::new(v),flags: 0, PC: 0, IrqH_addr: 0, EPC: 0, verbose: v};

    //init default irq_handler
    c.mem.setPrivileged(true);
    let DEFAULT_irq = Definitions::DEFAULT_IRQH;
    c.set_IrqHPC(4);
    c.mem.store(4, DEFAULT_irq.len(),&DEFAULT_irq);
    c.mem.protect(DEFAULT_irq.len() as u32+ 4);
    c.mem.setPrivileged(false);

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
     *  Set the range of protected addresses from 0 to proct_high
     * 
     *  ARGS:
     *  
     *   proct_high: Highest address of the reserved space
     * 
     */
    pub fn protect_mem(&mut self, protect_high: u32) {
        self.mem.protect(protect_high);
    }

    /**
     * Sets the IrqH PC for exceptions
     * 
     * ARGS:
     * 
     * irqpc: The address to jump to on interrupt
     */
    pub fn set_IrqHPC(&mut self, irqpc: u32) {
        self.IrqH_addr = irqpc
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

            //check if FIN_FLAG is set
            if self.flags & 1<< Definitions::FIN_FLAG != 0 {
                if self.verbose { println!("FIN_FLAG set; Flags={:08x}",self.flags) }
                break;
            }

        }

        if self.verbose {
            println!("Finished execution in T={} s",st.elapsed().as_secs_f64())
        }

    }


    fn run_handoff(&mut self, PC: u32) {

        //avoid weird tuples in vec::align_to, we know it'll always be aligned
        let code = Definitions::from_word(self.mem.load(PC,4));

        if PC == self.IrqH_addr +4 { panic!("past"); }

        if self.verbose {
            print!("Code: 0x{:01$x?} ",code,8);
            println!("at PC=0x{:01$x?}",PC,8);
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


    fn handoff_R(&mut self,code: u32,) {

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
            0b001001 => {self.reg[31] = self.PC; self.PC = rs-4;},//jalr
            0b001000 => {self.PC = rs-4},//jr
            0b010000 => {self.reg[rd] = self.HI;},//mfhi
            0b010010 => {self.reg[rd] = self.LO;},//mflo
            0b010001 => {self.HI = rs;},//mthi
            0b010011 => {self.LO = rs;},//mtlo


            _ => {panic!("Unrecognized R type func {:x}",func);}

        }

        if res == 0 { self.flags |= 1 << Definitions::Z_FLAG } else {self.flags &= 1<<Definitions::Z_FLAG};
        if (res as i32) < 0 {self.flags |= 1 << Definitions::S_FLAG} else {self.flags &= 1<<Definitions::S_FLAG}

    }


    fn handoff_I(&mut self, code: u32) {

        //special instruction: RFE
        if code == 0x42000010 {

            if self.verbose { println!("\tRFE:EPC={:08x}",self.EPC); }

            //panic if we are not privileged
            if  !(self.flags & (0x0000 | 1<< Definitions::MODE_FLAG) == 1) { panic!("Tried to use privileged instruction 0x{:08x} but the mode bitflag was not set to 1; Flags=0x{:08x}",code, self.flags); }
            //restore PC
            self.PC = self.EPC;
            //disable privileged
            self.flags &= !(1<<Definitions::MODE_FLAG);
            self.mem.setPrivileged(false);

            return;

        }

        //special instruction: hlt
        if code == 0x42000001 {

            let privileged = !(self.flags & (0x0000 | 1<< Definitions::MODE_FLAG) == 1);

            if self.verbose { println!("\tHLT: privilege status: {};",privileged ); }

            //panic if we are not privileged
            if  !privileged { panic!("Tried to use privileged instruction 0x{:08x} but the mode bitflag was not set to 1; Flags=0x{:08x}",code, self.flags); }
            self.flags |= 1<<Definitions::FIN_FLAG;

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
            0b001010 => {println!("slti");}, //slti
            0b001001 => {println!("sltiu");},//sltiu
            0b011001 => {println!("lhi");},//lhi
            0b011000 => {println!("llo");},//llo
            0b000100 => { if rs == self.reg[rt] {let jtarg = (self.PC) as i32 + ((imm as i32) <<2); self.PC = jtarg as u32;}; },//beq
            0b000101 => { if rs != self.reg[rt] {let jtarg = (self.PC) as i32 + ((imm as i32) <<2); self.PC = jtarg as u32;}; },//bne
            0b000111 => { if rs > 0             {let jtarg = (self.PC) as i32 + ((imm as i32) <<2); self.PC = jtarg as u32;}; },//bgtz
            0b000110 => { if rs <= self.reg[rt] {let jtarg = (self.PC) as i32 + ((imm as i32) <<2); self.PC = jtarg as u32;}; },//blez
            0b100000 => {self.reg[rt] = Definitions::from_byte(self.mem.load(rs+imm, 1) );},//lb
            0b100100 => {self.reg[rt] = Definitions::from_byte(self.mem.load(rs+imm, 1) );},//lbu
            0b100001 => {self.reg[rt] = Definitions::from_half(self.mem.load(rs+imm, 2) );},//lh
            0b100101 => {self.reg[rt] = Definitions::from_half(self.mem.load(rs+imm, 2) );},//lhu
            0b100011 => {self.reg[rt] = Definitions::from_word(self.mem.load(rs+imm, 4) );}//lw
            0b101000 => {
                let b = self.reg[rt];
                let v = vec![b as u8;1];

                self.mem.store((rs+imm) as usize, 1, &v);
            },//sb
            0b101001 => {
                let b = self.reg[rt];
                let v = vec![(b >> 8) as u8, (b & 0x00ff) as u8];

                self.mem.store((rs+imm) as usize, 2, &v);
            },//sh
            0b101011 => {
                let b = self.reg[rt];
                let v = vec![(b & 0xff000000 >> 24) as u8, (b & 0x00ff0000 >> 16) as u8,(b & 0x0000ff00 >> 8) as u8, (b & 0x000000ff) as u8];

                self.mem.store((rs+imm) as usize, 4, &v);
            },//sw


            _ => { panic!("Unrecognized I type func {:x}",func) }



        }

    }


    fn handoff_J(&mut self,code: u32) {

        //special instruction, syscall
        if code == 0x68000000 {

            if self.verbose { println!("\tSyscall; v0={}, v1={}", self.reg[2], self.reg[3]); }

            //save current pc, jump to IrqH, set privileged flag
            self.EPC = self.PC;
            self.PC = self.IrqH_addr-4;
            self.flags |= 1 << Definitions::MODE_FLAG;
            self.mem.setPrivileged(true);
            return;
        }

        let func          = (code & 0xfc000000) >> 26;
        let jump_target   = (code & !0xfc000000) << 2 ;

        println!("\tJ type: func=0x{:02x} jump_target=0x{:08x}",func,jump_target);

        match func {
            0b000010 => {self.PC = jump_target-4;}
            0b000011 => {self.reg[31] = self.PC; self.PC = jump_target-4;}

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
fn OOB_access() {
    let mut c: Core = new(true);

    let base = 0x04;
    c.mem.store(base, 4, &[0x24,0x08,0x01,0]); //addiu $at, $0, 256 
    c.mem.store(base+4, 4, &[0x81,0x01,0,0]);  //lb $at, 0($at)
    //mem_size < 0($at), crash

    c.PC = base as u32;

    c.run();
}

#[test]
#[should_panic]
fn unprivileged_rfe() {
    let mut c: Core = new(true);

    let code = [0x42, 0x00, 0x00, 0x10]; //rfe
    c.mem.store(4, 4, &code);
    c.run_handoff(0x4);
}
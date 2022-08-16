/**
 *  FLAG FORMAT
 *  0 1 2 3 4 ...
 *  Z|S| |M|F|
 */

pub const Z_FLAG: u32 = 1;
pub const S_FLAG: u32 = 1 << 1;
pub const INTERR_FLAG: u32 = 1 << 2;
pub const IENABLE_FLAG: u32 = 1 << 3;
pub const PRVILEGED_FLAG: u32 = 1 << 4;
pub const FIN_FLAG: u32 = 1 << 5;

//DEFAULT_IRQH CODE:

pub const DEFAULT_IRQH: [u8; 172] = [
    0x24, 0x1a, 0x00, 0x01, //'addiu 26, 0, 1'
    0x10, 0x5a, 0x00, 0x0a, //'beq 2, 26, printint'
    0x24, 0x1a, 0x00, 0x02, //'addiu 26, 0, 2'
    0x10, 0x5a, 0x00, 0x0d, //'beq 2, 26, printfloat'
    0x24, 0x1a, 0x00, 0x03, //'addiu 26, 0, 3'
    0x10, 0x5a, 0x00, 0x11, //'beq 2, 26, printdouble'
    0x24, 0x1a, 0x00, 0x04, //'addiu 26, 0, 4'
    0x10, 0x5a, 0x00, 0x15, //'beq 2, 26, printstring'
    0x24, 0x1a, 0x00, 0x0a, //'addiu 26, 0, 10'
    0x10, 0x5a, 0x00, 0x20, //'beq 2, 26, stop'
    0x24, 0x1a, 0x00, 0x0b, //'addiu 26, 0, 11'
    0x10, 0x5a, 0x00, 0x11, //'beq 2, 26, printstring'
    0x24, 0x01, 0x80, 0x00, //'addiu 1, 0, 32768'
    0x00, 0x01, 0x0c, 0x00, //'sll 1, 1, 16'
    0x34, 0x3a, 0x00, 0x00, //'ori  26, 1, 0'
    0xa3, 0x40, 0x00, 0x04, //'sb 0, 4(26)'
    0x08, 0x00, 0x00, 0x23, //'j print'
    0x24, 0x01, 0x80, 0x00, //'addiu 1, 0, 32768'
    0x00, 0x01, 0x0c, 0x00, //'sll 1, 1, 16'
    0x34, 0x3a, 0x00, 0x00, //'ori  26, 1, 0'
    0x24, 0x1b, 0x00, 0x01, //'addiu 27, 0, 1'
    0xa3, 0x5b, 0x00, 0x04, //'sb 27, 4(26)'
    0x08, 0x00, 0x00, 0x23, //'j print'
    0x24, 0x01, 0x80, 0x00, //'addiu 1, 0, 32768'
    0x00, 0x01, 0x0c, 0x00, //'sll 1, 1, 16'
    0x34, 0x3a, 0x00, 0x00, //'ori  26, 1, 0'
    0x24, 0x1b, 0x00, 0x02, //'addiu 27, 0, 2'
    0xa3, 0x5b, 0x00, 0x04, //'sb 27, 4(26)'
    0x08, 0x00, 0x00, 0x23, //'j print'
    0x24, 0x01, 0x80, 0x00, //'addiu 1, 0, 32768'
    0x00, 0x01, 0x0c, 0x00, //'sll 1, 1, 16'
    0x34, 0x3a, 0x00, 0x00, //'ori  26, 1, 0'
    0x24, 0x1b, 0x00, 0x03, //'addiu 27, 0, 3'
    0xa3, 0x5b, 0x00, 0x04, //'sb 27, 4(26)'
    0x08, 0x00, 0x00, 0x23, //'j print'
    0x24, 0x01, 0x80, 0x00, //'addiu 1, 0, 32768'
    0x00, 0x01, 0x0c, 0x00, //'sll 1, 1, 16'
    0x34, 0x3a, 0x00, 0x00, //'ori  26, 1, 0'
    0x8c, 0x9b, 0x00, 0x00, //'lw 27, 0(4)'
    0xaf, 0x5b, 0x00, 0x00, //'sw 27, 0(26)'
    0x08, 0x00, 0x00, 0x29, //'j exitirq'
    0x42, 0x00, 0x00, 0x01, //'rfe'
    0x42, 0x00, 0x00, 0x10, //'hlt'
];

pub const STACKSIZE: u32 = 512; // 512b stack

pub mod RegNames {

    #[allow(dead_code)]
    pub const ZERO: usize = 0;
    #[allow(dead_code)]
    pub const AT: usize = 1;
    #[allow(dead_code)]
    pub const V0: usize = 2;
    #[allow(dead_code)]
    pub const V1: usize = 3;
    #[allow(dead_code)]
    pub const A0: usize = 4;
    #[allow(dead_code)]
    pub const A1: usize = 5;
    #[allow(dead_code)]
    pub const A2: usize = 6;
    #[allow(dead_code)]
    pub const A3: usize = 7;
    #[allow(dead_code)]
    pub const T0: usize = 8;
    #[allow(dead_code)]
    pub const T1: usize = 9;
    #[allow(dead_code)]
    pub const T2: usize = 10;
    #[allow(dead_code)]
    pub const T3: usize = 11;
    #[allow(dead_code)]
    pub const T4: usize = 12;
    #[allow(dead_code)]
    pub const T5: usize = 13;
    #[allow(dead_code)]
    pub const T6: usize = 14;
    #[allow(dead_code)]
    pub const T7: usize = 15;
    #[allow(dead_code)]
    pub const S0: usize = 16;
    #[allow(dead_code)]
    pub const S1: usize = 17;
    #[allow(dead_code)]
    pub const S2: usize = 18;
    #[allow(dead_code)]
    pub const S3: usize = 19;
    #[allow(dead_code)]
    pub const S4: usize = 20;
    #[allow(dead_code)]
    pub const S5: usize = 21;
    #[allow(dead_code)]
    pub const S6: usize = 22;
    #[allow(dead_code)]
    pub const S7: usize = 23;
    #[allow(dead_code)]
    pub const S8: usize = 24;
    #[allow(dead_code)]
    pub const S9: usize = 25;
    #[allow(dead_code)]
    pub const K0: usize = 26;
    #[allow(dead_code)]
    pub const K1: usize = 27;
    #[allow(dead_code)]
    pub const GP: usize = 28;
    #[allow(dead_code)]
    pub const SP: usize = 29;
    #[allow(dead_code)]
    pub const FP: usize = 30;
    #[allow(dead_code)]
    pub const RA: usize = 31;
}

pub mod OP {

    /*
        OP contains full instructions for special operations, like
        NOP or RFE.

        Inner modules R, I and J contain *only opcodes* for their
        respective operation types
    */
    pub const NOP: u32 = 0x00000000;
    pub const RFE: u32 = 0x42000001;
    pub const HLT: u32 = 0x42000010;

    pub const SYSCALL: u32 = 0x68000000;

    pub mod R {

        pub const ADD: u32 = 0b100000;
        pub const ADDU: u32 = 0b100001;
        pub const AND: u32 = 0b100100;
        pub const NOR: u32 = 0b100111;
        pub const OR: u32 = 0b100101;
        pub const SUB: u32 = 0b100010;
        pub const SUBU: u32 = 0b100011;
        pub const XOR: u32 = 0b100110;
        pub const SLT: u32 = 0b101010;
        pub const SLTU: u32 = 0b101001;
        pub const DIV: u32 = 0b011010;
        pub const DIVU: u32 = 0b011011;
        pub const MULT: u32 = 0b011000;
        pub const MULTU: u32 = 0b011001;
        pub const SLL: u32 = 0b000000;
        pub const SRA: u32 = 0b000011;
        pub const SRAV: u32 = 0b000111;
        pub const SRLV: u32 = 0b000110;
        pub const JARL: u32 = 0b001001;
        pub const JR: u32 = 0b001000;
        pub const MFHI: u32 = 0b010000;
        pub const MFLO: u32 = 0b010010;
        pub const MTHI: u32 = 0b010001;
        pub const MTLO: u32 = 0b010011;
    }

    pub mod I {

        pub const ADDI: u32 = 0b001000;
        pub const ADDIU: u32 = 0b001001;
        pub const ANDI: u32 = 0b001100;
        pub const ORI: u32 = 0b001101;
        pub const XORI: u32 = 0b001110;
        pub const SLTI: u32 = 0b001010;
        pub const SLTIU: u32 = 0b001011;
        pub const LHI: u32 = 0b011001;
        pub const LLO: u32 = 0b011000;
        pub const BEQ: u32 = 0b000100;
        pub const BNE: u32 = 0b000101;
        pub const BGTZ: u32 = 0b000111;
        pub const BLEZ: u32 = 0b000110;
        pub const LB: u32 = 0b100000;
        pub const LBU: u32 = 0b100100;
        pub const LH: u32 = 0b100001;
        pub const LHU: u32 = 0b100101;
        pub const LW: u32 = 0b100011;
        pub const SB: u32 = 0b101000;
        pub const SH: u32 = 0b101001;
        pub const SW: u32 = 0b101011;
    }

    pub mod J {

        pub const J: u32 = 0b000010;
        pub const JAL: u32 = 0b000011;
    }
}

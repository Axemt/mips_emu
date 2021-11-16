
#![allow(non_snake_case)]

use std::convert::TryInto;

pub type Byte = u8;
pub type Half = u16;
pub type Word = u32;

#[derive(Debug)]
pub struct RelfHeader32 {

    pub e_ident_MAG: u32,
    pub e_ident_CLASS: u8,
    pub e_ident_DATA: u8,
    pub e_ident_VERSION: u8,
    pub e_ident_OSABI: u8,
    pub e_ident_ABIVERSION: u8,
    e_ident_EIPAD : std::vec::Vec<u8>, //7B :(  not used, so this dirty hack with vec works
    pub e_type: u16,
    pub e_machine: u16,
    pub e_version: u32,
    pub e_entry: u32,
    pub e_phoff: u32,
    pub e_shoff: u32,
    pub e_flags: u32,
    pub e_ehsize: u16,
    pub e_phentsize: u16,
    pub e_phnum: u16,
    pub e_shentsize: u16,
    pub e_shnum: u16,
    pub e_shstrndx: u16
    
}
    impl RelfHeader32{

        pub fn from_tuple(tuple: (u32,u8,u8,u8,u8,u8,std::vec::Vec<u8>,u16,u16,u32,u32,u32,u32,u32,u16,u16,u16,u16,u16,u16)) -> RelfHeader32 {

            return RelfHeader32{
                e_ident_MAG: tuple.0, 
                e_ident_CLASS: tuple.1, 
                e_ident_DATA: tuple.2, 
                e_ident_VERSION: tuple.3, 
                e_ident_OSABI: tuple.4, 
                e_ident_ABIVERSION: tuple.5, 
                e_ident_EIPAD: tuple.6, 
                e_type: tuple.7, 
                e_machine: tuple.8,
                e_version: tuple.9, 
                e_entry: tuple.10,
                e_phoff: tuple.11,
                e_shoff: tuple.12,
                e_flags: tuple.13,
                e_ehsize: tuple.14,
                e_phentsize: tuple.15, 
                e_phnum: tuple.16,
                e_shentsize: tuple.17, 
                e_shnum: tuple.18,
                e_shstrndx: tuple.19
            }
         
         }

    }



#[derive(Debug)]
pub struct SectionHeader32 {

    pub p_type: u32,
    pub p_offset: u32,
    pub p_vaddr: u32,
    pub p_paddr: u32,
    pub p_filesz: u32,
    pub p_memsz: u32,
    pub p_flags: u32,
    p_align: u32 // unused

}

    impl SectionHeader32 {

        pub fn from_tuple(tuple: (u32,u32,u32,u32,u32,u32,u32,u32)) -> SectionHeader32{

            return SectionHeader32 {
                p_type: tuple.0, 
                p_offset: tuple.1, 
                p_vaddr: tuple.2,
                p_paddr: tuple.3,
                p_filesz: tuple.4, 
                p_memsz: tuple.5,
                p_flags: tuple.6,
                p_align: tuple.7
            }
        }

    }

/**
 *  Converts a slice of N u8/bytes into an array [u8; N]
 * 
 *      let arr = &[1,2,3];
 *      assert_eq!([1, 2, 3, 0], from_sizeN::<4>(arr));
 * 
 *  unused positions are filled with 0
 *  if N < contents.len(), the array is truncated
 * 
 *  GENERICS:
 *  
 *  const N: the size of the output array
 * 
 *  ARGS:
 * 
 *  contents: the array to convert
 * 
 *  RETURNS:
 * 
 *  the output array [u8; N]
 */
pub fn from_sizeN<const N: usize>(contents: &[u8]) -> [u8; N] {

    let mut tmpVec = contents.to_vec();
    tmpVec.resize(N,0);
    let res = tmpVec.try_into().unwrap();

    return res;

}

/**
 *  Converts a sequence of four u8/bytes into a u32/word as follows
 * 
 *      got = Definitions::to_word(&[1,255,1,255]);
 *      assert_eq!(0x01ff01ff, got);
 * 
 *  The array with bytes [x, y, z, t] is transformed into 0Xxxyyzztt
 * 
 *  ARGS:
 * 
 *  contents: the array to convert
 * 
 *  RETURNS:
 * 
 *  the converted word in u32
 */
#[inline(always)]
pub fn from_word(contents: &[u8]) -> u32 {
    let word: u32 = contents[3] as u32 | (contents[2] as u32) << 8 | (contents[1] as u32) << 16 | (contents[0]as u32) << 24;
    return word;
}


/**
 *  Converts a sequence of two u8/bytes into a u32/word as follows
 * 
 *      got = Definitions::to_half(&[1,255]);
 *      assert_eq!(0x000001ff, got);
 * 
 *  The array with bytes [x, y] is transformed into 0X0000xxyy
 * 
 *  ARGS:
 * 
 *  contents: the array to convert
 * 
 *  RETURNS:
 * 
 *  the converted halfword in u32
 */
#[inline(always)]
pub fn from_half(contents: &[u8]) -> u32 {
    let word: u32 = (contents[1] as u32)| (contents[0] as u32) << 8;
    return word
}

/**
 *  Converts a sequence of two u8/bytes into a u32/word as follows
 * 
 *      got = Definitions::to_byte(&[255]);
 *      assert_eq!(0x000000ff, got);
 * 
 *  The array with byte [x] is transformed into 0X000000xx
 * 
 *  ARGS:
 * 
 *  contents: the array to convert
 * 
 *  RETURNS:
 * 
 *  the converted byte in u32
 */
#[inline(always)]
pub fn from_byte(contents: &[u8]) -> u32 {
    let word: u32 = contents[0] as u32;
    
    return word;
}

/**
 *  FLAG FORMAT
 *  0 1 2 3 4 ...
 *  Z|S| |M|F|
 */

pub const Z_FLAG:      u32 = 0;
pub const S_FLAG:      u32 = 1;
pub const INTERR_FLAG: u32 = 2;
pub const MODE_FLAG:   u32 = 3;
pub const FIN_FLAG:    u32 = 4;


//DEFAULT_IRQH CODE:

pub const DEFAULT_IRQH: [u8; 156] = [
    
    0x24, 0x1a, 0x00, 0x01, //'addiu 26, 0, 1'
    0x10, 0x5a, 0x00, 0x0b, //'beq 2, 26, printint'
    0x24, 0x1a, 0x00, 0x02, //'addiu 26, 0, 2'
    0x10, 0x5a, 0x00, 0x0e, //'beq 2, 26, printfloat'
    0x24, 0x1a, 0x00, 0x03, //'addiu 26, 0, 3'
    0x10, 0x5a, 0x00, 0x12, //'beq 2, 26, printdouble'
    0x24, 0x1a, 0x00, 0x04, //'addiu 26, 0, 4'
    0x10, 0x5a, 0x00, 0x16, //'beq 2, 26, printstring'
    0x24, 0x1a, 0x00, 0x0a, //'addiu 26, 0, 10'
    0x10, 0x5a, 0x00, 0x1d, //'beq 2, 26, stop'
    0x24, 0x1a, 0x00, 0x0b, //'addiu 26, 0, 11'
    0x10, 0x5a, 0x00, 0x12, //'beq 2, 26, printstring'
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
    0xaf, 0x44, 0x00, 0x00, //'sw 4, 0(26)'
    0x08, 0x00, 0x00, 0x25, //'j exitirq'
    0x42, 0x00, 0x00, 0x01, //'rfe'
    0x42, 0x00, 0x00, 0x10  //'hlt'
];


/**
 *  TESTS
 */


#[test]
fn conversions() {
    let mut got = from_half(&[0,1]);
    assert_eq!(0x00000001, got);

    got = from_half(&[1,255]);
    assert_eq!(0x000001ff, got);

    got = from_word(&[0,0,0,1]);
    assert_eq!(0x00000001,got);

    got = from_word(&[255,0,255,0]);
    assert_eq!(0xff00ff00,got);
}

#[test]
fn from_size() {

    //extends with empty
    let mut arr: &[u8] = &[1,2,3];
    assert_eq!([1, 2, 3, 0], from_sizeN::<4>(arr));

    //truncates
    arr = &[23, 4, 5, 6, 9]; //len 5
    assert_eq!([23, 4, 5, 6], from_sizeN::<4>(arr));
}


#![allow(non_snake_case)]

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
pub fn from_byte(contents: &[u8]) -> u32 {
    let word: u32 = contents[0] as u32;
    
    return word;
}


pub const Z_FLAG: u32 = 0;
pub const S_FLAG: u32 = 1;
pub const FIN_FLAG: u32 = 4;


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


#![allow(non_snake_case)]

use std::convert::TryInto;

pub type Byte = u8;
pub type Half = u16;
pub type Word = u32;

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
 * Converts the bytes of an u32 to the signed representation 
 * of the number, still in u32 format
 * 
 * NOTE: Since the value is still returned as u32, it has no sign
 * and therefore is considered positive by default
 * 
 *          n = -70;
 *          conv = to_signed(0xffffffba);
 *          //0xffffffba = -70
 *          assert_eq!(n, -(conv as i32));
 * 
 * ARGS:
 * 
 *  n: the number to convert
 * 
 * RETURNS
 * 
 *  signed representation of this number as u32
 */
pub fn to_signed<const size: u8>(n: u32) -> u32 
{
    //2's complement
    let _ret = (!n).overflowing_add(1).0;

    //there's probably a better way than using match, parametric types restricting to primitives? how?
    match size {
        8 =>  { return (_ret as u8) as u32; }
        16 => { return (_ret as u16) as u32; }
        32 => { return _ret; }
        _ => { panic!("Unknown size required: {}", size); }

    }

}


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

    let mut n: i32 = -1;
    //0xffffffff = -1
    let mut conv = to_signed::<32>(0xffffffff);
    assert_eq!(n,-(conv as i32));

    n = -70;
    conv = to_signed::<32>(0xffffffba);
    //0xffffffba = -70
    assert_eq!(n, -(conv as i32));

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

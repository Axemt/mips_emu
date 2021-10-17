use super::MemoryMapped;
use std::io;
use std::io::Read;

pub struct Keyboard {
    pub range_lower: u32,
    pub range_upper: u32,
    buffer: Vec<u8>,
    mode: u8
}

impl MemoryMapped for Keyboard {

    fn read(&mut self, dir: u32, size: usize) -> & [u8] {

        self.buffer.clear();


        if dir >= self.range_lower+4 {

            io::stdin().read(&mut self.buffer).unwrap();

        } else {

            panic!("Tried to read from non-readable address {:08x} in device 'Keyboard'", dir);

        }

        return &self.buffer[..size];

        

    }

    fn write(&mut self, dir: usize, _size: usize, contents: &[u8]) -> () {
        
        if dir as u32 == self.range_lower { self.mode = contents[0]; return }
        
        panic!("Tried to write to non-writeable address {:08x} in device 'Keyboard'",dir); 
    }
    
}

/**
 * Creates a Keyboard device implementing MemoryMapped and with the following
 * address ranges
 * 
 * 
 * 0x80000008..0x8000000b: Read content
 * 
 * 
 * 0x8000000c..0x8000000f: Mode
 */
pub fn new() -> Keyboard {

    return Keyboard { range_lower: 0x80000008, range_upper: 0x8000000f, buffer: Vec::<u8>::new(), mode: 0 }

}

#[test]
fn integrity() {
    let k = new();

    assert!(k.range_upper > k.range_lower);
}

#[test]
fn read_setup_K() {
    
    let mut k: Keyboard = new();

    //set mode to 0
    k.write(0x80000008, 4, &[0]);

    //cannot test actual reading atm since we get from std::io::stdin() directly
}

#[test]
#[should_panic]
fn write_K() {
    let mut k: Keyboard = new();

    k.write( (k.range_lower+4) as usize, 4, &[0;4]);
}

#[test]
#[should_panic]
fn read_mode_range_K() {
    let mut k: Keyboard = new();

    k.write(0x80000008, 4, &[0]);
    k.read(0x80000008, 1);
}
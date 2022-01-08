use super::MemoryMapped;
use std::io;
use std::io::Read;
use super::super::Definitions::Utils::from_sizeN;

pub struct Keyboard {
    pub range_lower: u32,
    pub range_upper: u32,
    buffer: Vec<u8>,
    mode: u8
}

impl MemoryMapped for Keyboard {

    fn read(&mut self, dir: u32, size: usize) -> & [u8] {

        self.buffer.clear();


        if dir < self.range_lower+3 {

            io::stdin().read(&mut self.buffer).unwrap();
            //remove intro character
            self.buffer.pop();

        } else {

            panic!("Tried to read from non-readable address 0x{:08x} in device 'Keyboard'", dir);

        }

        &self.buffer[..size]

        

    }

    fn write(&mut self, dir: usize, size: usize, contents: &[u8]) -> () {
        
        if (dir as u32 == self.range_lower+4) && (dir as u32 + size as u32) < self.range_upper { self.mode = contents[0]; return }
        
        panic!("Tried to write to non-writeable address 0x{:08x} in device 'Keyboard'",dir); 
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

    Keyboard { range_lower: 0x80000008, range_upper: 0x8000000f, buffer: Vec::<u8>::new(), mode: 0 }

}

#[test]
fn integrity() {
    let k = new();

    assert!(k.range_upper > k.range_lower);
}

#[test]
fn write_mode_K() {
    let mut k: Keyboard = new();

    k.write( (k.range_lower+4) as usize, 1, &[0;4]);
}

#[test]
#[should_panic]
fn read_mode_K() {
    let mut k: Keyboard = new();

    k.write(0x80000008, 4, &[0]);
    k.read(0x80000008, 1);
}
use super::MemoryMapped;

#[derive(Copy, Clone)]

pub struct Console {
    pub range_lower: u32,
    pub range_upper: u32,
    mode: u8

}


impl MemoryMapped for Console {


    fn read(& mut self, dir: u32, _size: usize) -> &[u8] { panic!("Tried to read from non-readable device 'Console' at address {:08x}",dir); }

    fn write(&mut self, dir: usize, size: usize, contents: &[u8]) { 
        

        //if writing to lower address, print
        if dir+size-1 <= self.range_lower as usize + 3 {
            match self.mode {
                0 => { for i in contents { print!("{}",*i as u32);  } println!() } //print int
                1 => { for i in contents { print!("{}",*i as f32);  } println!() } //print float
                2 => { for i in contents { print!("{}",*i as u64);  } println!() } //print double
                3 => { for i in contents { print!("{}",*i as char); } println!()} //print string
                _ => { panic!("Console: Unknown print mode {}", self.mode); }
            }
        }

        //write a byte to mode
        else { self.mode = contents[0]; }
        
    }

}

/**
 * Creates a Console device implementing MemoryMapped and with the following
 * address ranges
 * 
 * 
 * 0x80000000..0x80000003: Printable content
 * 
 * 
 * 0x80000004..0x80000007: Mode
 */
pub fn new() -> Console {

    return Console { range_lower: 0x80000000, range_upper: 0x80000007, mode: 0};
}

#[test]
fn integrity() {
    let c = new();

    assert!(c.range_upper > c.range_lower);
}

#[test]
fn write_C() {
    
    let mut c: Console = new();

    //set mode to 3, string
    c.write(0x80000004, 1, &[3]);
    c.write(0x80000000,4,b"abcd");
}

#[test]
#[should_panic]
fn read_C() {
    let mut c: Console = new();

    c.read(0, 4);
}
use super::super::Definitions::Errors::MemError;
use super::super::Definitions::Utils::from_sizeN;
use super::MemoryMapped;

#[derive(Copy, Clone)]

pub struct Console {
    pub range_lower: u32,
    pub range_upper: u32,
    mode: u8,
}

impl MemoryMapped for Console {
    fn read(&mut self, dir: u32, _size: usize) -> Result<&[u8], MemError> {
        Err(MemError::MappedDeviceError(String::from(format!(
            "Tried to read from non-readable device 'Console' at address {:08x}",
            dir
        ))))
    }

    fn write(&mut self, dir: usize, size: usize, contents: &[u8]) -> Result<(), MemError> {
        //if writing to lower address, print
        if dir + size - 1 <= self.range_lower as usize + 3 {
            print!("[CON]:");
            match self.mode {
                0 => {
                    print!("{}", u32::from_be_bytes(from_sizeN::<4>(contents)))
                } //print int
                1 => {
                    print!("{}", f32::from_be_bytes(from_sizeN::<4>(contents)))
                } //print float
                2 => {
                    print!("{}", f64::from_be_bytes(from_sizeN::<8>(contents)))
                } //print double
                3 => {
                    for i in contents {
                        print!("{}", *i as char);
                    }
                } //print string
                _ => {
                    return Err(MemError::MappedDeviceError(String::from(format!(
                        "Console: Unknown print mode {}",
                        self.mode
                    ))))
                }
            }
            println!()
        }
        //write a byte to mode
        else {
            self.mode = contents[0];
        }

        Ok(())
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
    Console {
        range_lower: 0x80000000,
        range_upper: 0x80000007,
        mode: 0,
    }
}

#[test]
fn integrity() {
    let c = new();

    assert!(c.range_upper > c.range_lower);
}

#[test]
fn write_modes_C() {
    let mut c: Console = new();

    //set mode to 3, string
    c.write(0x80000004, 1, &[3]).unwrap();
    c.write(0x80000000, 4, b"abcd").unwrap();

    //set mode to 1, float
    c.write(0x80000004, 1, &[1]).unwrap();
    let x: f32 = 12.34;
    c.write(0x80000000, 4, &x.to_be_bytes()).unwrap();

    //set mode to double
    c.write(0x80000004, 1, &[2]).unwrap();
    let y: f64 = 3.1415926535;
    c.write(0x80000000, 4, &y.to_be_bytes()).unwrap();
}

#[test]
#[should_panic]
fn read_C() {
    let mut c: Console = new();

    c.read(0, 4).unwrap();
}

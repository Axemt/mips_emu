pub mod Console;
pub mod Keyboard;

pub trait MemoryMapped {

    fn read(& mut self, dir: u32, size: usize) -> &[u8];

    fn write(&mut self, dir: usize, size: usize, contents: &[u8]) -> ();

}
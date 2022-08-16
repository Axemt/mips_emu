use super::Definitions::Errors::MemError;

pub mod Console;
pub mod Interruptor;
pub mod Keyboard;

pub trait MemoryMapped {
    fn read(&mut self, dir: u32, size: usize) -> Result<&[u8], MemError>;

    fn write(&mut self, dir: usize, size: usize, contents: &[u8]) -> Result<(), MemError>;
}

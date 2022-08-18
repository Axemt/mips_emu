use crate::libs::Definitions::Errors::ExecutionError;
use crate::libs::Memory::Memory;
use std::error::Error;

pub trait PipelinedWithHeldMemory {
    fn tick_with_mem(&mut self, mem: &mut Memory) -> Result<(), ExecutionError>;
}

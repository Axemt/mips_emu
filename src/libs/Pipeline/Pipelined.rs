use crate::libs::Definitions::Errors::ExecutionError;
use std::error::Error;

pub trait Pipelined {
    fn tick(&mut self) -> Result<(), ExecutionError>;
}

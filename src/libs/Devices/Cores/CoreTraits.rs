use crate::libs::Definitions::Errors::{ExecutionError, HeaderError};

pub trait Runnable {
    fn run(&mut self) -> Result<(), ExecutionError>;
    fn load_RELF(&mut self, path: &str) -> Result<(), HeaderError>;
    fn load_bin(&mut self, path: &str, entry: u32) -> Result<(), std::io::Error>;
}

pub trait Privileged {
    fn set_privilege(&mut self, privilege: bool);
}

#[derive(Debug)]
pub enum HeaderError {
    MagicError,
    ArchError,
    PermExecError(String),
    IOError(String)
}


impl std::fmt::Display for HeaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
          HeaderError::MagicError => write!(f, "ELF Magic Number not found"),
          HeaderError::ArchError => write!(f, "This file's Architecture is not compatible with the machine"),
          HeaderError::PermExecError(emsg) => write!(f, "{emsg}"),
          HeaderError::IOError(emsg) => write!(f, "{emsg}")
        }
      }
}

impl From<std::io::Error> for HeaderError {
       fn from(e: std::io::Error) -> Self {
         HeaderError::IOError(String::from("Propagated io::Error: {e.to_string()}"))
       }
     }

impl std::error::Error for HeaderError {}

#[derive(Debug)]
pub enum MemError {
  PermError(String),
  MappedDeviceError(String)
}

impl std::fmt::Display for MemError {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    match self {
      MemError::PermError(emsg) => write!(f, "{emsg}"),
      MemError::MappedDeviceError(emsg) => {write!(f, "{emsg}")}
    }
  }
}

impl std::error::Error for MemError {}


#[derive(Debug)]
pub enum ExecutionError {
  PrivilegeError(String),
  UnrecognizedOPError(String),
  MemError(String)
}

impl From<MemError> for ExecutionError {
  fn from(e: MemError) -> Self {
    ExecutionError::MemError(String::from("Propagated MemError: {e.to_string()}"))
  }
}

impl std::fmt::Display for ExecutionError {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    match self {
      ExecutionError::PrivilegeError(iname) => { write!(f, "Tried to use privileged instruction {iname} but the mode bitflag was not set") },
      ExecutionError::UnrecognizedOPError(emsg) => { write!(f, "{emsg}") },
      ExecutionError::MemError(emsg) => { write!(f, "{emsg}") }
    }
  }
}

impl std::error::Error for ExecutionError {}
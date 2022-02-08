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
         HeaderError::IOError(String::from(e.to_string()))
       }
     }

impl std::error::Error for HeaderError {}
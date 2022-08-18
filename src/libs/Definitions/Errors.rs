use std::fmt::Formatter;

#[allow(dead_code)]
#[derive(Debug)]
pub enum HeaderError {
    MagicError,
    ArchError,
    PermExecError(String),
    IOError(String),
}

impl std::fmt::Display for HeaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            HeaderError::MagicError => write!(f, "ELF Magic Number not found"),
            HeaderError::ArchError => write!(
                f,
                "This file's Architecture is not compatible with the machine"
            ),
            HeaderError::PermExecError(emsg) => write!(f, "{emsg}"),
            HeaderError::IOError(emsg) => write!(f, "{emsg}"),
        }
    }
}

impl From<std::io::Error> for HeaderError {
    fn from(e: std::io::Error) -> Self {
        HeaderError::IOError(format!("Propagated io::Error: {}", e.to_string()))
    }
}

impl std::error::Error for HeaderError {}

#[derive(Debug)]
pub enum MemError {
    PermError(u32, u32, usize),
    MappedDeviceError(String),
}

impl std::fmt::Display for MemError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
      MemError::PermError(range_hi, range_lo, addr) => write!(f, "{}", format!("Tried to access protected region range [0x{:08x}..0x{:08x}] at address 0x{:08x}", range_hi, range_lo, addr)),
      MemError::MappedDeviceError(emsg) => {write!(f, "{emsg}")}
    }
    }
}

impl std::error::Error for MemError {}

#[derive(Debug)]
pub enum ExecutionError {
    PrivilegeError(String),
    UnrecognizedOPError(String),
    MemError(String),
    RegisterError(String),
}

impl From<MemError> for ExecutionError {
    fn from(e: MemError) -> Self {
        ExecutionError::MemError(format!("Propagated MemError: {}", e.to_string()))
    }
}

impl From<RegisterError> for ExecutionError {
    fn from(e: RegisterError) -> Self {
        ExecutionError::RegisterError(format!("Propagated RegisterError: {}", e.to_string()))
    }
}

impl std::fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ExecutionError::PrivilegeError(iname) => {
                write!(
                    f,
                    "Tried to use privileged instruction {iname} but the mode bitflag was not set"
                )
            }
            ExecutionError::UnrecognizedOPError(emsg) => {
                write!(f, "{emsg}")
            }
            ExecutionError::MemError(emsg) => {
                write!(f, "{emsg}")
            }
            ExecutionError::RegisterError(emsg) => {
                write!(f, "{emsg}")
            }
        }
    }
}

impl std::error::Error for ExecutionError {}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum RegisterError {
    LockedWithHandle(usize, u32),
    NotOwned(usize, u32),
}

impl std::fmt::Display for RegisterError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RegisterError::LockedWithHandle(owner_handle, regno) => {
                write!(
                    f,
                    "The register {regno} is locked by owner instruction with timestamp {owner_handle}"
                )
            }
            RegisterError::NotOwned(owner_handle, regno) => {
                write!(
                    f,
                    "The instruction tried to write to register {regno}, which is owned by the instruction with timestamp {owner_handle}"
                )
            }
        }
    }
}

impl std::error::Error for RegisterError {}

#[test]
fn error_fmt() {
    println!("{}", HeaderError::MagicError);
    println!("{}", HeaderError::ArchError);
    println!("{}", HeaderError::PermExecError(String::from("")));
    println!("{}", HeaderError::IOError(String::from("")));

    println!("{}", MemError::MappedDeviceError(String::from("")));
    println!("{}", MemError::PermError(1, 2, 3));

    println!("{}", ExecutionError::MemError(String::from("")));
    println!("{}", ExecutionError::PrivilegeError(String::from("")));
    println!("{}", ExecutionError::UnrecognizedOPError(String::from("")));
}

#[test]
fn error_from() {
    #[allow(unused_variables)]
    let e: HeaderError = std::io::Error::new(std::io::ErrorKind::Other, "error!").into();
    #[allow(unused_variables)]
    let e2: ExecutionError = MemError::PermError(1, 2, 3).into();
}

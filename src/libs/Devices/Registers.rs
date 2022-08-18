use crate::libs::Definitions::Arch::RegNames;
use crate::libs::Definitions::Errors::RegisterError;

type InUse = bool;
type Owner = Option<usize>;
type Register = (u32, InUse, Owner);

pub const HI_IDENT: u32 = 33u32;
pub const LO_IDENT: u32 = 34u32;

#[derive(Debug)]
pub struct Registers {
    reg: [Register; 32],
    HI: Register,
    LO: Register,
    verbose: bool,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Available {
    pub value: u32,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct SuccessfulOwn {
    pub register_number: u32,
}

impl Registers {
    pub fn new(verbose: bool) -> Registers {
        Registers {
            reg: [(0, false, None); 32],
            HI: (0, false, None),
            LO: (0, false, None),
            verbose,
        }
    }

    pub fn fetch(&self, register_number: u32) -> Result<Available, RegisterError> {
        let reg_contents = if register_number == HI_IDENT {
            self.HI
        } else if register_number == LO_IDENT {
            self.LO
        } else if register_number == 0 {
            return Ok(Available { value: 0 });
        } else {
            self.reg[register_number as usize]
        };

        if reg_contents.1 == false {
            Ok(Available {
                value: reg_contents.0,
            })
        } else {
            match reg_contents.2 {
                Some(owner) => Err(RegisterError::LockedWithHandle(owner, register_number)),
                None => {
                    panic!(
                        "[REG]::Internal : Register {} marked as in-use but no owner assigned!",
                        register_number
                    )
                }
            }
        }
    }

    pub fn lock_for_write(
        &mut self,
        register_number: u32,
        accessor_id: usize,
    ) -> Result<SuccessfulOwn, RegisterError> {
        let reg_contents = if register_number == HI_IDENT {
            self.HI
        } else if register_number == LO_IDENT {
            self.LO
        } else {
            self.reg[register_number as usize]
        };

        if reg_contents.1 == true {
            match reg_contents.2 {
                Some(owner) => Err(RegisterError::LockedWithHandle(owner, register_number)),
                None => {
                    panic!(
                        "[REG]::Internal : Register {} marked as in-use but no owner assigned!",
                        register_number
                    )
                }
            }
        } else {
            self.reg[register_number as usize] = (reg_contents.0, true, Some(accessor_id));
            if self.verbose {
                println!(
                    "\t[REG]: id {} successfully locked register {}",
                    accessor_id, register_number
                )
            }
            Ok(SuccessfulOwn { register_number })
        }
    }

    pub fn write_and_unlock(
        &mut self,
        register_number: u32,
        contents: u32,
        accessor_id: usize,
    ) -> Result<(), RegisterError> {
        let reg_contents = if register_number == HI_IDENT {
            self.HI
        } else if register_number == LO_IDENT {
            self.LO
        } else {
            self.reg[register_number as usize]
        };

        if reg_contents.1 && reg_contents.2 == Some(accessor_id) {
            if register_number != 0 {
                if self.verbose {
                    println!(
                        "\t[REG]: id {} wrote [{}] to register {} and unlocked",
                        accessor_id, contents, register_number
                    )
                }
                self.reg[register_number as usize] = (contents, false, None);
            }
            Ok(())
        } else {
            match reg_contents.2 {
                Some(owner) => Err(RegisterError::NotOwned(owner, register_number)),
                None => {
                    panic!(
                        "[REG]::Internal : Register {} marked as in-use but no owner assigned",
                        register_number
                    )
                }
            }
        }
    }
}

#[test]
fn owned_access() {
    let mut r: Registers = Registers::new(true);

    let id = 0xB00B1E5;
    let reg = RegNames::T4 as u32;
    let contents = 2022;

    r.lock_for_write(reg, id).unwrap();

    r.write_and_unlock(reg, contents, id).unwrap();

    let content = r.fetch(reg);
    match content {
        Ok(Available { value: n }) => {
            assert_eq!(n, contents);
        }
        Err(eobj) => {
            panic!("{}", eobj)
        }
    }
}

#[test]
#[should_panic]
fn lock_owned_reg() {
    let mut r: Registers = Registers::new(true);

    let id = 0xAB00BA;
    let reg = RegNames::S0 as u32;

    r.lock_for_write(reg, id).unwrap();

    let non_owner_id = 0xBEEEF;
    match r.lock_for_write(reg, non_owner_id) {
        Ok(_) => {}
        Err(eobj) => {
            panic!("{}", eobj)
        }
    }
}

#[test]
#[should_panic]
fn write_locked_reg() {
    let mut r: Registers = Registers::new(true);

    let id = 0xBAF;
    let reg = RegNames::RA as u32;
    let contents = 995599;

    r.lock_for_write(reg, id).unwrap();

    let non_owner_id = 0xB0F;
    match r.write_and_unlock(reg, contents, non_owner_id) {
        Ok(_) => {}
        Err(eobj) => {
            panic!("{}", eobj)
        }
    }
}

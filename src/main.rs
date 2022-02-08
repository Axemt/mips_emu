#![feature(bigint_helper_methods)]
#![feature(toowned_clone_into)]
mod libs;
use libs::Core;
use std::panic;

//import macro for pack/unpack
#[macro_use]
extern crate structure;
extern crate clap;

use clap::{Arg,App};

fn main() {

    let matches = App::new("Mips Runtime")
                    .version("0.90 built on Nov 23, 2021" )
                    .author("Axemt <github.com/Axemt>")
                    .about("A MIPS R3000 32b emulator")
                    .arg(Arg::with_name("File")
                             .short("f")
                             .long("File")
                             .value_name("FILEPATH")
                             .help("File to load to memory")
                             .takes_value(true)
                             .required(true)
                            )
                    .arg(Arg::with_name("Verbose")
                             .help("Set the verbose flag to show internal processing of the emulator")
                             .short("v")
                             .long("Verbose")
                             .value_name("V")
                             .takes_value(false)
                            )
                    .arg(Arg::with_name("Entry")
                             .help("Set a custom entrypoint (Required for .bin files); If using a hex value, prefix with '0x'")
                             .short("e")
                             .long("Entry")
                             .value_name("E")
                             .takes_value(true)
                            )

                    .get_matches();

    let v = matches.is_present("Verbose");
    let filepath = matches.value_of("File").unwrap();


    let mut cpu = Box::<Core::Core>::new(Core::new(v));

    if filepath.ends_with(".relf") {

        match cpu.load_RELF(filepath) {
            Err(eobj) => { panic!("{eobj}") }
            _ => {}
        }

    } else { //raw .bin file

        if matches.is_present("Entry") {


            let s = matches.value_of("Entry").unwrap();

            let entry: u32;

            if s.starts_with("0x") | s.starts_with("0X") {
                entry = u32::from_str_radix(s.trim_start_matches("0x").trim_start_matches("0X"),16).unwrap();
            } else {
                entry = s.parse::<u32>().unwrap();
            }

            match cpu.load_bin(filepath,entry) {
                Err(eobj) => { panic!("{eobj}") }
                _ => {}
            }

        } else {
            panic!("A raw binary file was detected, but no entrypoint was given");
        }

    }

    match cpu.run() {
        Err(eobj) => { panic!("EXECUTION FAILED: {eobj}") }
        _ => {}
    }

}

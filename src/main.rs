#![feature(bigint_helper_methods)]

mod libs;
use libs::Core::Core;
use std::panic;

//import macro for pack/unpack
#[macro_use]
extern crate structure;
extern crate clap;

use clap::Parser;

#[derive(Parser, Debug)]
#[clap(
    author = "Axemt <github.com/Axemt>",
    version = "0.92 built on Feb 21, 2022",
    about = "A MIPS R3000 32b emulator",
    long_about = None
)]
struct Args {
    #[clap(short, long, help = "File to load to memory", required=true)]
    filepath : String,
    #[clap(short, long, help = "Set the verbose flag to show internal processing of the emulator", takes_value = false)]
    verbose : bool,

    //TODO: See args.entry block
    #[clap(short, long, help = "Set a custom entrypoint (Required for .bin files); If using a hex value, prefix with '0x'", required = false, default_value = "")]
    entry : String
}

#[cfg(not(tarpaulin_include))]
fn main() {

    let args = Args::parse();

    let v = args.verbose;
    let filepath = args.filepath;


    let mut cpu = Box::<Core>::new(Core::new(v));

    if filepath.ends_with(".relf") {

        match cpu.load_RELF(&filepath) {
            Err(eobj) => { panic!("{eobj}") }
            _ => {}
        }

    } else { //raw .bin file

        //TODO: Any way to go into this block if args.entry is present instead of comparing with an arbitrary default?
        if args.entry != "" {


            let s = args.entry;

            let entry: u32;

            if s.starts_with("0x") | s.starts_with("0X") {
                entry = u32::from_str_radix(s.trim_start_matches("0x").trim_start_matches("0X"),16).unwrap();
            } else {
                entry = s.parse::<u32>().unwrap();
            }

            match cpu.load_bin(&filepath,entry) {
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

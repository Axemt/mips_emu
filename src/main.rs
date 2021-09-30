mod libs;
use libs::Core;

//import macro for pack/unpack
#[macro_use]
extern crate structure;
extern crate clap;

use clap::{Arg,App};

fn main() {

    let matches = App::new("Mips Runtime")
                    .version("0.5")
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
                            
                    .get_matches();

    let v = matches.is_present("Verbose");
    let filepath = matches.value_of("File").unwrap();
    
    let mut cpu = Core::new(v);

    cpu.load_RELF(filepath);

    cpu.run();

}
use super::Definitions::RELFHeaders::{RelfHeader32,SectionHeader32};
use super::Definitions::Errors::{HeaderError, MemError};
use super::Definitions::Utils::{Byte, Half, Word};
use super::Devices::MemoryMapped;

use std::fs::File;
use std::io::Read;
use std::io;

extern crate structure;

pub struct Memory {

    mem_array: Vec<Byte>,
    mem_size: usize,
    mode_privilege: bool,
    verbose: bool,
    protected_ranges: Vec<(u32, u32)>,
    devices: Vec<(u32, u32, Box<dyn MemoryMapped>)>,
    
}


 impl Memory {

    /**
     * Initializes the memory object
     * 
     * ARGS:
     * 
     *  v: Verbose flag
     * **/
    pub fn new( v: bool) -> Memory{
    
        Memory {
             mem_array: vec![0;0],
             mem_size: 0,
             protected_ranges: Vec::<(u32, u32)>::new(),
             mode_privilege: false,
             verbose: v, 
             devices: Vec::<(u32, u32, Box<dyn MemoryMapped>)>::new()
            }
    }

        /**
     *  Adds the range proct_low .. proct_high to the set of protected address ranges
     * 
     *  ARGS:
     * 
     *  proct_low: Lowest address of the reserved range
     * 
     *  proct_high: Highest address of the reserved range
     * 
     */
    pub fn protect(&mut self, proct_low: u32, proct_high: u32) {

        if self.verbose { println!("[MEM]: Protecting range [0x{:08x}..0x{:08x}]", proct_low, proct_high); }

        self.protected_ranges.push( (proct_low, proct_high) );
    }

    /**
     * Changes the level of privilege access of the memory
     * 
     * ARGS:
     * 
     *  m: The new level of permission
     */
    pub fn set_privileged(&mut self,m: bool) {
        if self.verbose { println!("[MEM]: Changed privilege mode to {}", m); }
        self.mode_privilege = m;
    }
    

    pub fn map_device(&mut self, range_lower: u32, range_upper: u32, device: Box<dyn MemoryMapped>) {

        if self.verbose { println!("[MEM]: Mapping device to range [0x{:08x}..0x{:08x}]", range_lower, range_upper); }

        self.devices.push( (range_lower, range_upper, device) );

    }

    /**
     * Extends memory allocation by alloc bytes
     * 
     * ARGS:
     * 
     *  alloc: Amount to extend memory by
    */
    fn extend_mem(& mut self, alloc: usize) {

        //reduce useless extensions, extend by word mimimum
        // extend by appending with empty vec of alloc length, avoid using resize     
        self.mem_array.extend(&vec![0; if alloc > 4 {alloc} else {4} ]);

        self.mem_size += alloc;
        if self.verbose { println!("[MEM]: extend_mem adding {} bytes. New size is: {}. Highest address is: 0x{:x}",alloc,self.mem_size,self.mem_size) }
    }


    /**
     * Extends memory allocation on a fast way, can ONLY BE USED ONCE, when loading an executable
     * 
     * ARGS:
     * 
     *  alloc: Amount to extend memory by 
    */
    fn extend_mem_FAST(&mut self, alloc: usize) {

        //fast track, avoid Vec::resize at all costs
        self.mem_array = vec![0; alloc];
        self.mem_size = alloc;

        if self.verbose { println!("[MEM]: extend_mem_FAST adding {} bytes. New size is: {}. Highest address is: 0x{:x}",alloc,self.mem_size,self.mem_size) }
    }

    /**
     * Returns a slice of memory
     * 
     * ARGS:
     * 
     *  dir: memory address to return
     * 
     *  size: amount of bytes to return after dir
     * 
     * RETURNS:
     * 
     *  pointer to slice
    */
    pub fn load(& mut self,dir: u32 , size: usize) -> Result<&[Byte], MemError> {

        let contents: &[u8];

        for elem in & self.protected_ranges {

            let prot_lo = elem.0;
            let prot_high = elem.1;

            if dir < prot_high && dir >= prot_lo && !self.mode_privilege { 
                return Err(MemError::PermError(prot_lo, prot_high,dir as usize) ) 
            }

        }   

        let d = dir as usize;

        //TODO: extend mem *after* the device check so we don't extend if not necessary
        //fake having a 4GB memory by dynamically extending on "OOB" accesses
        if d+size > self.mem_size { self.extend_mem(d+size-self.mem_size); }
        

        for (dev_lower, dev_upper, device) in & mut self.devices {
            

            //check if in range of a device
            if dir >= *dev_lower && dir <= *dev_upper { 

                if self.verbose { println!("[MEM]: Read access to Memory Mapped Device at address 0x{:08x}; Handing off...", dir); }

                contents =  device.read(dir, size)? ;
                return Ok(contents);
            }

        }
        
        //get pointer to slice
        contents = &self.mem_array[d..d+size]; 

        if self.verbose { println!("[MEM]: loading: align={} dir={:08x?} contents={:x?}",size,dir,contents); }

        
        Ok(contents)
    }

    /**
     * Stores size bytes of contents in memory address dir
     * 
     * ARGS:
     * 
     *  dir: memory address to store
     * 
     *  size: amount of bytes to store
     * 
     *  contents: bytes to store
    */
    pub fn store(& mut self, dir: usize ,size: usize, contents: & [Byte]) -> Result<(), MemError> {

        let d = dir as u32;

        //check protection
        for elem in & mut self.protected_ranges {

            let prot_lo = elem.0;
            let prot_high = elem.1;

            if d < prot_high && d >= prot_lo && self.mode_privilege == false {
                return Err(MemError::PermError(prot_lo, prot_high, dir) );
            }

        }
        

        for elem in & mut self.devices {
            
            let dev_lower = elem.0;
            let dev_upper = elem.1;

            //check if in range of a device
            if d >= dev_lower && d <= dev_upper { 

                if self.verbose { println!("[MEM]: Write access to Memory Mapped Device at address 0x{:08x} with contents {:?}; Handing off...", dir,contents); }

                elem.2.write(dir, size, contents)?;

                return Ok(());
            }
        }

        //extend dynamically
        if dir+size >= self.mem_size { self.extend_mem(dir+size - self.mem_size);}

        if self.verbose { println!("[MEM]: storing: align={} dir={:08x?} contents={:02x?}", size, dir, contents); }
        // copy into mem array, consume elements
        let mut to_insert: Byte;
        let mut byte: usize = 0;
        for i in 0..size {
            
            //always insert fixed size, fill with 0 if empty
            if i < contents.len() {
                to_insert = contents[i];
            } else {
                to_insert = 0;
            }
            self.mem_array[dir+byte] = to_insert;
            byte += 1;

        }

        Ok(())

    }

    /** 
     * Loads a binary file byte by byte into memory starting from 0x00000000
     * 
     * Note that this overwrites memory and ignores reserved ranges for writing
    */
    pub fn load_bin(& mut self, bin: &str) -> Result<(), io::Error> {
        let mut fBuffer: Vec<u8> = Vec::new();
        File::open(bin)?.read_to_end(& mut fBuffer)?;
        
        //read contents of file into buffer


        //raw copy into mem
        self.extend_mem_FAST(fBuffer.len());
        
        for (offset, b) in fBuffer.iter().enumerate() {
            self.mem_array[offset] = *b
        }

        Ok(())
    }


    pub fn load_RELF(& mut self, RELF: &str)  -> Result<u32, HeaderError> {


        //read file to descriptor, allocate buffer as Vec
        let mut fBuffer: Vec<Byte> = Vec::new();
        //read contents of file into buffer
        File::open(RELF)?.read_to_end(& mut fBuffer)?;

        // unpack
        let relf_header: RelfHeader32 = structure!(">I5B7s2H5I6H").unpack(&fBuffer[0..52])?.into();

        if self.verbose {
            println!("found RelfHeader32!:\n\t{:x?}",relf_header);
        }

        //sanity checks
        if relf_header.e_ident_MAG != 0x7f454c46 { return Err(HeaderError::MagicError) }
        //assert_eq!(relf_header.e_ident_MAG,0x7f454c46,"ELF Magic Number not found"); //has magic number
        if relf_header.e_ident_CLASS != 0x01 { return Err(HeaderError::ArchError) }
        //assert_eq!(relf_header.e_ident_CLASS,0x01,"The file is not a 32b architecture"); // is 32b
        if relf_header.e_type != 0x02 { return Err(HeaderError::PermExecError(String::from("This file is not an executable"))) }
        //assert_eq!(relf_header.e_type,0x02,"The file is not an executable"); // is executable
        if relf_header.e_machine != 0x08 { return Err(HeaderError::ArchError) }
        //assert_eq!(relf_header.e_machine,0x08,"This executable's architecture is not MIPS"); // is mips architecture


        //unpack
        let prog_header: SectionHeader32 = structure!(">8I").unpack(&fBuffer[52..(52+relf_header.e_phentsize) as usize])?.into();

        if self.verbose {
            println!("found PROGRAM SectionHeader32!:\n\t{:x?}",prog_header);
        }

        if prog_header.p_flags != 0x05000000 { return Err(HeaderError::PermExecError(String::from("The text segment is not Readable and Executable"))) }
        //assert_eq!(prog_header.p_flags,0x05000000,"This text segment is not Readable and Executable"); // Readable, Executable
        if prog_header.p_type != 0x1 { return Err(HeaderError::PermExecError(String::from("The text segment is not Loadable"))) }
        //assert_eq!(prog_header.p_type,0x1,"This text segment is not Loadable"); //is loadable segment


        //unpack
        let data_header: SectionHeader32 = structure!(">8I").unpack(&fBuffer[(52+relf_header.e_phentsize as usize)..(52+2*relf_header.e_phentsize as usize)])?.into();


        if self.verbose {
            println!("found DATA SectionHeader32!:\n\t{:x?}",data_header);
        }

        //sanity checks
        if data_header.p_flags != 0x06000000 { return Err(HeaderError::PermExecError(String::from("The data segment is not Readable and Writeable"))) }
        //assert_eq!(data_header.p_flags,0x06000000,"This data segment is not Readable and Writeable"); // Readable, Writeable
        if data_header.p_type != 0x1 { return Err(HeaderError::PermExecError(String::from("The data segment is not Loadable"))) }
        //assert_eq!(data_header.p_type,0x1,"This data segment is not Loadable"); //is loadable segment


        //extract code and data raws

        // check if there is a data segment or it is empty
        let data_raw;
        if data_header.p_memsz > 0 {
            data_raw = &fBuffer[(52+4+data_header.p_offset+data_header.p_offset) as usize..];
        } else {
            data_raw = &[0;0];
        }

        let code_raw = &fBuffer[52+prog_header.p_offset as usize..(52+prog_header.p_offset+prog_header.p_memsz) as usize];

        //set up memory size
        //get max of both and extend, then use extend_mem
        let to_alloc = std::cmp::max(prog_header.p_paddr+prog_header.p_memsz, data_header.p_paddr + data_raw.len() as u32) as usize;


        //We CANNOT use extend_mem_FAST because it'll overwrite the default irqH. only allowed in load_bin because we don't care there
        self.extend_mem(to_alloc);
        //if the data segment exists, load it

        //copy to memory
        let mut offset = prog_header.p_paddr as usize;

        for byte in code_raw {
            self.mem_array[offset] = *byte;
            offset += 1;
        }

        offset = data_header.p_paddr as usize;
        for byte in data_raw {
            self.mem_array[offset] = *byte;
            offset += 1;
        }

        Ok(relf_header.e_entry)        
    }

}



/**
 *  TESTS
 */



#[test]
fn loading() {

    let mut m: Memory = Memory::new(true);
    match m.load_RELF("testbins/parsing_more.s.relf") {
        Ok(ept) => assert_eq!(ept,0x00400000),
        Err(eobj) => panic!("{eobj}")
    }
    
    drop(m);

    let mut m: Memory = Memory::new(false);
    match m.load_RELF("testbins/testingLS.s.relf") {
        Ok(ept) => assert_eq!(ept,0x00400000),
        Err(eobj) => panic!("{eobj}")
    }
    
}

#[test]
fn load_store() {
    let mut m = Memory::new(true);

    //store as word...
    //also implicitly extends mem dynamically
    m.store(0x00020000, 4, &[0,1]).unwrap();

    //..but retrieve as half
    let got = m.load(0x00020000, 2).unwrap();

    assert_eq!([0,1],got);
}

#[test]
fn extend_mem_all() {

    let mut m: Memory = Memory::new(true);

    m.extend_mem_FAST(0x700000);

    assert_eq!(m.mem_size, m.mem_array.len());
    assert_eq!(m.mem_array.len(), 0x700000);

    m = Memory::new(true);

    m.extend_mem(0x80);

    assert_eq!(m.mem_size,m.mem_array.len());
    assert_eq!(m.mem_array.len(),0x80);

}

#[test]
#[should_panic]
fn unprivileged_protected_access() {

    let mut m: Memory = Memory::new(true);

    m.extend_mem_FAST(0x0000ff00);
    m.protect(0,0x0000fC00);

    //correct access
    let got = m.load(0x0000fd00, 4).unwrap();


    //access to protected -> panic
    let mut m2: Memory = Memory::new(true);
    m2.extend_mem_FAST(0x0000ff00);
    m2.protect(0,0x0000fC00);
    m2.store(0x0000AA00, 4, got).unwrap();
}

#[test]
fn privileged_protected_access() {

       let mut m: Memory = Memory::new(true);

       m.extend_mem_FAST(0x0000ff00);
       m.protect(0,0x0000fC00);
       m.set_privileged(true);
       
       //correct access   
       m.store(0x0000AA00, 4, &[0x69, 0x69, 0x69, 0x66]).unwrap();
       let _ = m.load(0x0000AA00, 4); 
    
}

#[test]
fn device_access() {
    use super::Devices;

    let mut m: Memory = Memory::new(true);
    let c = Box::new(Devices::Console::new() );
    let k = Box::new(Devices::Keyboard::new() );

    m.map_device(c.range_lower, c.range_upper, c);
    m.map_device(k.range_lower, k.range_upper, k);

    //write to Console
    m.store(0x80000000, 4, b"abcd").unwrap();

    //store (mode) from Keyboard
    m.store(0x8000000c, 1, &[1]).unwrap();

}

use super::Definitions::{RelfHeader32,SectionHeader32};
use super::Definitions::{Byte, Half, Word};
use super::Definitions;
use super::Devices::MemoryMapped;
use std::panic;
use std::fs::File;
use std::io::Read;

pub struct Memory {

    mem_array: Vec<Byte>,
    mem_size: usize,
    mode_privilege: bool,
    verbose: bool,
    protected_ranges: Vec<(u32, u32)>,
    devices: Vec<(u32, u32, Box<dyn MemoryMapped>)>,
    
}

/**
     * Initializes the memory object
     * 
     * ARGS:
     * 
     *  v: Verbose flag
     * **/
pub fn new( v: bool) -> Memory{

    return Memory {
         mem_array: vec![0;0],
         mem_size: 0,
         protected_ranges: Vec::<(u32, u32)>::new(),
         mode_privilege: false,
         verbose: v, 
         devices: Vec::<(u32, u32, Box<dyn MemoryMapped>)>::new()
        };

}

 impl Memory {

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
    pub fn setPrivileged(&mut self,m: bool) {
        if self.verbose { println!("[MEM]:Changed privilege mode to {}", m); }
        self.mode_privilege = m;
    }
    

    pub fn mapDevice(&mut self, range_lower: u32, range_upper: u32, device: Box<dyn MemoryMapped>) {

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
    fn extend_mem(&mut self, mut alloc: usize) {


        //reduce useless extensions, extend by word mimimum
        if alloc < 4 { alloc = 4; }
        

        // extend by appending with empty vec of alloc length        
        self.mem_array.extend(&vec![0;alloc]);

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
    pub fn load(&mut self,dir: u32 , size: usize) -> &[Byte] {

        let contents: &[u8];

        for elem in & mut self.protected_ranges {

            let prot_lo = elem.0;
            let prot_high = elem.1;

            if dir < prot_high && dir >= prot_lo && self.mode_privilege == false { panic!("Tried to access protected region range [0x{:08x}..0x{:08x}] at address 0x{:08x}",prot_lo, prot_high,dir); }

        }   

        let d = dir as usize;

        //fake having a 4GB memory by dynamically extending on "OOB" accesses
        if d+size > self.mem_size { self.extend_mem(d+size-self.mem_size); }
        

        for elem in & mut self.devices {
            
            let dev_lower = elem.0;
            let dev_upper = elem.1;

            //check if in range of a device
            if dir >= dev_lower && dir <= dev_upper { 

                if self.verbose { println!("[MEM]: Read access to Memory Mapped Device at address {:08x}; Handing off...", dir); }

                contents =  elem.2.read(dir, size) ;
                return contents;
            }

        }
        
        //get pointer to slice
        contents = &self.mem_array[d..d+size]; 

        if self.verbose { println!("[MEM]: loading: align={} dir={:08x?} contents={:x?}",size,dir,contents); }

        return contents;

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
    pub fn store(&mut self, dir: usize ,size: usize, contents: &[Byte]) {

        let d = dir as u32;

        //check protection
        for elem in & mut self.protected_ranges {

            let prot_lo = elem.0;
            let prot_high = elem.1;

            if d < prot_high && d >= prot_lo && self.mode_privilege == false { panic!("Tried to access protected region range [0x{:08x}..0x{:08x}] at address 0x{:08x}",prot_lo, prot_high,dir); }

        }
        //extend dynamically
        if dir+size >= self.mem_size { self.extend_mem(dir+size - self.mem_size);}

        for elem in & mut self.devices {
            
            let dev_lower = elem.0;
            let dev_upper = elem.1;

            //check if in range of a device
            if d >= dev_lower && d <= dev_upper { 

                if self.verbose { println!("[MEM]: Write access to Memory Mapped Device at address {:08x} with contents {:?}; Handing off...", dir,contents); }

                elem.2.write(dir, size, contents);

                return;
            }


        }


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
            byte = byte + 1;

        }

    }

    /** 
     * Loads a binary file byte by byte into memory starting from 0x00000000
     * 
     * Note that this overwrites memory and ignores reserved ranges for writing
    */
    pub fn load_bin(&mut self, bin: &str) {
        let mut f = File::open(bin).unwrap();
        let mut fBuffer: Vec<u8> = Vec::new();

        let fLen = f.metadata().unwrap().len();
        
        //read contents of file into buffer
        f.read_to_end(&mut fBuffer).unwrap();


        //raw copy into mem
        self.extend_mem_FAST(fLen as usize);

        let mut offset = 0;
        for b in &fBuffer {
            self.mem_array[offset] = *b;
            offset += 1;
        }
    }


    pub fn load_RELF(&mut self, RELF: &str)  -> u32 {


        //read file to descriptor, allocate buffer as Vec
        let mut f = File::open(RELF).unwrap();
        let mut fBuffer: Vec<Byte> = Vec::new();

        
        //read contents of file into buffer
        f.read_to_end(&mut fBuffer).unwrap();

        // unpack
        let s_relf_header = structure!(">I5B7s2H5I6H");
        let tuple_relf_header = s_relf_header.unpack(&fBuffer[0..52]).unwrap();

        //cast
        let relf_header: RelfHeader32 = RelfHeader32::from_tuple(tuple_relf_header);

        if self.verbose {
            println!("found RelfHeader32!:\n\t{:x?}",relf_header);
        }

        //sanity checks
        assert_eq!(relf_header.e_ident_MAG,0x7f454c46,"ELF Magic Number not found"); //has magic number
        assert_eq!(relf_header.e_ident_CLASS,0x01,"The file is not a 32b architecture"); // is 32b
        assert_eq!(relf_header.e_type,0x02,"The file is not an executable"); // is executable
        assert_eq!(relf_header.e_machine,0x08,"This executable's architecture is not MIPS"); // is mips architecture


        //unpack
        let s_header = structure!(">8I");
        let tuple_prog_header = s_header.unpack(&fBuffer[52..(52+relf_header.e_phentsize) as usize]).unwrap();
        //cast
        let prog_header : SectionHeader32 = SectionHeader32::from_tuple(tuple_prog_header);

        if self.verbose {
            println!("found PROGRAM SectionHeader32!:\n\t{:x?}",prog_header);
        }

        assert_eq!(prog_header.p_flags,0x05000000,"This text segment is not Readable and Executable"); // Readable, Executable
        assert_eq!(prog_header.p_type,0x1,"This text segment is not Loadable"); //is loadable segment


        //unpack
        let tuple_data_header = s_header.unpack(&fBuffer[(52+relf_header.e_phentsize as usize)..(52+2*relf_header.e_phentsize as usize)]).unwrap();
        //cast
        let data_header : SectionHeader32 = SectionHeader32::from_tuple(tuple_data_header);

        if self.verbose {
            println!("found DATA SectionHeader32!:\n\t{:x?}",data_header);
        }

        //sanity checks
        assert_eq!(data_header.p_flags,0x06000000,"This data segment is not Readable and Writeable"); // Readable, Writeable
        assert_eq!(data_header.p_type,0x1,"This data segment is not Loadable"); //is loadable segment


        //extract code and data raws

        // check if there is a data segment or it is empty
        let data_raw;
        if data_header.p_memsz > 0 {
            data_raw = &fBuffer[(52+4+data_header.p_offset+data_header.p_offset) as usize..];
        } else {
            data_raw = &[0;1];
        }

        let code_raw = &fBuffer[52+prog_header.p_offset as usize..(52+prog_header.p_offset+prog_header.p_memsz) as usize];

        //set up memory size
        //get max of both and extend, then use extend_mem
        let to_alloc = std::cmp::max(prog_header.p_paddr+prog_header.p_memsz as u32, data_header.p_paddr + data_raw.len() as u32);


        //We CANNOT use extend_mem_FAST because it'll overwrite the default irqH. only allowed in load_bin because we don't care there
        self.extend_mem(to_alloc as usize);
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

        return relf_header.e_entry;

        
    }

}



/**
 *  TESTS
 */



#[test]
fn loading() {
    
    let entry;

    let mut m: Memory = new(true);
    entry = m.load_RELF("src/libs/testbins/parsing_more.s.relf");

    assert_eq!(entry,0x00400000);
    
    drop(m);

    let mut m: Memory = new(false);
    let entry = m.load_RELF("src/libs/testbins/testingLS.s.relf");

    assert_eq!(entry,0x00400000);
}

#[test]
fn load_store() {
    let mut m = new(true);

    //store as word...
    //also implicitly extends mem dynamically
    m.store(0x00020000, 4, &[0,1]);

    //..but retrieve as half
    let got = m.load(0x00020000, 2);

    assert_eq!([0,1],got);
}

#[test]
fn extend_mem_all() {

    let mut m: Memory = new(true);

    m.extend_mem_FAST(0x700000);

    assert_eq!(m.mem_size, m.mem_array.len());
    assert_eq!(m.mem_array.len(), 0x700000);

    m = new(true);

    m.extend_mem(0x80);

    assert_eq!(m.mem_size,m.mem_array.len());
    assert_eq!(m.mem_array.len(),0x80);

}

#[test]
#[should_panic]
fn unprivileged_protected_access() {

    let mut m: Memory = new(true);

    m.extend_mem_FAST(0x0000ff00);
    m.protect(0,0x0000fC00);

    //correct access
    let got = m.load(0x0000fd00, 4);


    //access to protected -> panic
    let mut m2: Memory = new(true);
    m2.extend_mem_FAST(0x0000ff00);
    m2.protect(0,0x0000fC00);
    m2.store(0x0000AA00, 4, got);
}

#[test]
fn privileged_protected_access() {

       let mut m: Memory = new(true);

       m.extend_mem_FAST(0x0000ff00);
       m.protect(0,0x0000fC00);
       m.setPrivileged(true);
       
       //correct access   
       m.store(0x0000AA00, 4, &[0x69, 0x69, 0x69, 0x66]);
       let _ = m.load(0x0000AA00, 4); 
    
}

#[test]
fn device_access() {
    use super::Devices;

    let mut m: Memory = new(true);
    let c = Box::new(Devices::Console::new() );
    let k = Box::new(Devices::Keyboard::new() );

    m.mapDevice(c.range_lower, c.range_upper, c);
    m.mapDevice(k.range_lower, k.range_upper, k);

    //write to Console
    m.store(0x80000000, 4, b"abcd");

    //store (mode) from Keyboard
    m.store(0x8000000c, 1, &[1]);

}
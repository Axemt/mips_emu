use super::Definitions::{RelfHeader32,SectionHeader32};
//use super::Definitions::{to_byte,to_half,to_word};
use std::panic;
use std::fs::File;
use std::io::Read;

pub struct Memory {

    mem_array: Vec<u8>,
    mem_size: usize,

    verbose: bool
}

/**
     * Initializes the memory object
     * 
     * ARGS:
     * 
     *  v: Verbose flag
     * **/
pub fn new( v: bool) -> Memory{

    return Memory {mem_array: vec![0;0], mem_size: 0, verbose: v};

}

 impl Memory {
    

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
        if self.verbose { println!("extend_mem adding {} bytes. New size is: {}. Highest address is: 0x{:x}",alloc,self.mem_size,self.mem_size) }

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

        if self.verbose { println!("extend_mem_FAST adding {} bytes. New size is: {}. Highest address is: 0x{:x}",alloc,self.mem_size,self.mem_size) }
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
    pub fn load(&mut self,dir: u32 , size: usize) -> &[u8] {

        let d = dir as usize;

        //fake having a 4GB memory by dynamically extending on "OOB" accesses
        //if d+size > self.mem_size { self.extend_mem(d+size-self.mem_size); }

        if d+size > self.mem_size { panic!("Tried to access memory address 0x{:08x} but current memory size is 0x{:08x}. Out of bounds",d+size,self.mem_size); }
        
        //get pointer to slice
        let contents: &[u8] = &self.mem_array[d..d+size]; 

        if self.verbose { println!("\tloading: align={} dir={:08x?} contents={:x?}",size,dir,contents); }

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
    pub fn store(&mut self, dir: usize,size: usize, contents: &[u8]) {

        //extend dynamically
        if dir > self.mem_size { self.extend_mem(dir+size - self.mem_size);}


        if self.verbose { println!("\tstoring: align={} dir={:08x?} contents={:02x?}", size, dir, contents); }
        // copy into mem array, consume elements
        let mut to_insert: u8;
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
        let mut fBuffer: Vec<u8> = Vec::new();

        
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
        assert_eq!(relf_header.e_ident_MAG,0x7f454c46); //has magic number
        assert_eq!(relf_header.e_ident_CLASS,0x01); // is 32b
        assert_eq!(relf_header.e_type,0x02); // is executable
        assert_eq!(relf_header.e_machine,0x08); // is mips architecture


        //unpack
        let s_header = structure!(">8I");
        let tuple_prog_header = s_header.unpack(&fBuffer[52..(52+relf_header.e_phentsize) as usize]).unwrap();
        //cast
        let prog_header : SectionHeader32 = SectionHeader32::from_tuple(tuple_prog_header);

        if self.verbose {
            println!("found PROGRAM SectionHeader32!:\n\t{:x?}",prog_header);
        }

        assert_eq!(prog_header.p_flags,0x05000000); // Readable, Executable
        assert_eq!(prog_header.p_type,0x1); //is loadable segment


        //unpack
        let tuple_data_header = s_header.unpack(&fBuffer[(52+relf_header.e_phentsize as usize)..(52+2*relf_header.e_phentsize as usize)]).unwrap();
        //cast
        let data_header : SectionHeader32 = SectionHeader32::from_tuple(tuple_data_header);

        if self.verbose {
            println!("found DATA SectionHeader32!:\n\t{:x?}",data_header);
        }

        //sanity checks
        assert_eq!(data_header.p_flags,0x06000000); // Readable, Writeable
        assert_eq!(data_header.p_type,0x1); //is loadable segment


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
        //get max of both and extend, then use extend_mem_FAST
        let to_alloc = std::cmp::max(prog_header.p_paddr+prog_header.p_memsz as u32, data_header.p_paddr + data_raw.len() as u32);


        self.extend_mem_FAST(to_alloc as usize);
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
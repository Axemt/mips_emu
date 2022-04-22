#[derive(Debug)]
pub struct RelfHeader32 {

    pub e_ident_MAG: u32,
    pub e_ident_CLASS: u8,
    pub e_ident_DATA: u8,
    pub e_ident_VERSION: u8,
    pub e_ident_OSABI: u8,
    pub e_ident_ABIVERSION: u8,
    #[allow(dead_code)]
    e_ident_EIPAD : std::vec::Vec<u8>, //7B :(  not used, so this dirty hack with vec works
    pub e_type: u16,
    pub e_machine: u16,
    pub e_version: u32,
    pub e_entry: u32,
    pub e_phoff: u32,
    pub e_shoff: u32,
    pub e_flags: u32,
    pub e_ehsize: u16,
    pub e_phentsize: u16,
    pub e_phnum: u16,
    pub e_shentsize: u16,
    pub e_shnum: u16,
    pub e_shstrndx: u16
    
}
    impl RelfHeader32{

        pub fn from_tuple(tuple: (u32,u8,u8,u8,u8,u8,std::vec::Vec<u8>,u16,u16,u32,u32,u32,u32,u32,u16,u16,u16,u16,u16,u16)) -> RelfHeader32 {

            RelfHeader32{
                e_ident_MAG: tuple.0, 
                e_ident_CLASS: tuple.1, 
                e_ident_DATA: tuple.2, 
                e_ident_VERSION: tuple.3, 
                e_ident_OSABI: tuple.4, 
                e_ident_ABIVERSION: tuple.5, 
                e_ident_EIPAD: tuple.6, 
                e_type: tuple.7, 
                e_machine: tuple.8,
                e_version: tuple.9, 
                e_entry: tuple.10,
                e_phoff: tuple.11,
                e_shoff: tuple.12,
                e_flags: tuple.13,
                e_ehsize: tuple.14,
                e_phentsize: tuple.15, 
                e_phnum: tuple.16,
                e_shentsize: tuple.17, 
                e_shnum: tuple.18,
                e_shstrndx: tuple.19
            }
         
         }

    }

impl From<(u32,u8,u8,u8,u8,u8,std::vec::Vec<u8>,u16,u16,u32,u32,u32,u32,u32,u16,u16,u16,u16,u16,u16)> for RelfHeader32 {
    fn from(tpl: (u32,u8,u8,u8,u8,u8,std::vec::Vec<u8>,u16,u16,u32,u32,u32,u32,u32,u16,u16,u16,u16,u16,u16)) -> Self {
        RelfHeader32::from_tuple(tpl)
    }
}

#[derive(Debug)]
pub struct SectionHeader32 {

    pub p_type: u32,
    pub p_offset: u32,
    pub p_vaddr: u32,
    pub p_paddr: u32,
    pub p_filesz: u32,
    pub p_memsz: u32,
    pub p_flags: u32,
    #[allow(dead_code)]
    p_align: u32 // unused

}

    impl SectionHeader32 {

        pub fn from_tuple(tuple: (u32,u32,u32,u32,u32,u32,u32,u32)) -> SectionHeader32{

            SectionHeader32 {
                p_type: tuple.0, 
                p_offset: tuple.1, 
                p_vaddr: tuple.2,
                p_paddr: tuple.3,
                p_filesz: tuple.4, 
                p_memsz: tuple.5,
                p_flags: tuple.6,
                p_align: tuple.7
            }
        }

    }

impl From<(u32,u32,u32,u32,u32,u32,u32,u32)> for SectionHeader32 {
    fn from(tpl: (u32,u32,u32,u32,u32,u32,u32,u32)) -> Self {
        SectionHeader32::from_tuple(tpl)
    }
}
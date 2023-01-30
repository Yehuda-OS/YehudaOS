use super::Process;
use fs_rs::fs;

type ElfAddr = u64; // Unsigned program address
type ElfOff = u64; // Unsigned file offset
type ElfSection = u16; // Unsigned section index
type ElfVersym = u16; // Unsigned version symbol information
type ElfByte = u8;
type ElfHalf = u16;
type ElfSword = i32;
type ElfWord = u32;
type ElfSxword = i64;
type ElfXword = u64;

const EI_NIDENT: usize = 16;
const EI_MAG0: u8 = 0x7f;
const EI_MAG1: u8 = 'E' as u8;
const EI_MAG2: u8 = 'L' as u8;
const EI_MAG3: u8 = 'F' as u8;

#[repr(C)]
#[derive(Default)]
struct ElfEhdr {
    e_idnt: [u8; EI_NIDENT],
    e_type: u16,
    e_machine: u16,
    e_version: u32,
    e_entry: ElfAddr,
    e_phoff: ElfOff,
    e_shoff: ElfOff,
    e_flags: u32,
    e_ehsize: u16,
    e_phentsize: u16,
    e_phnum: u16,
    e_shentsize: u16,
    e_shnum: u16,
    e_shstrndx: u16,
}

fn get_header(file_id: u64) -> ElfEhdr {
    let mut header = ElfEhdr::default();
    let header_slice = unsafe {
        core::slice::from_raw_parts_mut(
            &mut header as *mut _ as *mut u8,
            core::mem::size_of::<ElfEhdr>(),
        )
    };

    unsafe {
        fs::read(file_id as usize, header_slice, 0);
    }

    header
}

pub fn load_process(file_id: u64) -> Process {
    let header = get_header(file_id);
    let p = Process {
        registers: super::Registers::default(),
        page_table: unsafe { crate::memory::PAGE_TABLE },
        stack_pointer: 0,
        instruction_pointer: header.e_entry,
        flags: 0,
    };

    p
}

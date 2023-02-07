use super::Process;
use crate::memory;
use core::fmt;
use fs_rs::fs;
use x86_64::{
    structures::paging::{PageSize, PageTableFlags, Size4KiB},
    VirtAddr,
};

/// Unsigned program address
type ElfAddr = u64;
/// Unsigned file offset
type ElfOff = u64;

const PROCESS_STACK_POINTER: u64 = 0x7000_0000_0000;

const EI_NIDENT: usize = 16;
const PT_LOAD: u32 = 1;

#[derive(Debug)]
pub struct OutOfMemory {}

impl fmt::Display for OutOfMemory {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "not enough memory to create a process")
    }
}

#[repr(C)]
#[derive(Default)]
struct ElfEhdr {
    e_idnt: [u8; EI_NIDENT],
    e_type: u16,
    e_machine: u16,
    e_version: u32,
    /// Entry point of the process
    e_entry: ElfAddr,
    /// Program header offset
    e_phoff: ElfOff,
    e_shoff: ElfOff,
    e_flags: u32,
    e_ehsize: u16,
    /// Program header entry size
    e_phentsize: u16,
    /// Program header entry count
    e_phnum: u16,
    e_shentsize: u16,
    e_shnum: u16,
    e_shstrndx: u16,
}

#[repr(C)]
#[derive(Default, Clone)]
struct ElfPhdr {
    p_type: u32,
    p_flags: u32,
    p_offset: ElfOff,
    p_vaddr: ElfAddr,
    p_paddr: ElfAddr,
    p_filesz: u64,
    p_memsz: u64,
    p_align: u64,
}

/// Returns the header of the ELF file.
///
/// # Arguments
/// - `file_id` - The ID of the ELF file.
fn get_header(file_id: u64) -> ElfEhdr {
    let mut header = ElfEhdr::default();
    // SAFETY: The header is of the size of `ElfEhdr`.
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

/// Returns an array of the program header entry.
///
/// # Arguments
/// - `file_id` - The ID of the ELF file.
/// - `header` - The header of the ELF file.
fn get_program_table(file_id: u64, header: &ElfEhdr) -> alloc::vec::Vec<ElfPhdr> {
    let mut buffer = alloc::vec![ElfPhdr::default(); header.e_phnum as usize];

    unsafe {
        fs::read(
            file_id as usize,
            core::slice::from_raw_parts_mut(
                buffer.as_mut_ptr() as *mut u8,
                buffer.len() * header.e_phentsize as usize,
            ),
            header.e_phoff as usize,
        );

        buffer
    }
}

/// Map a segment to a process' address space.
///
///  # Arguments
/// - `p` - The process' struct.
/// - `segment` - The segment to map.
fn map_segment(p: &Process, segment: &ElfPhdr) -> Result<(), OutOfMemory> {
    let flags =
        PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE | PageTableFlags::WRITABLE;
    let mut mapped = 0;
    let mut page;

    while mapped < segment.p_memsz {
        page = memory::page_allocator::allocate().ok_or(OutOfMemory {})?;
        // The page table should not be null because it is returned from the `create_page_table`
        // function.
        // If the file is valid, the virtual address should not be already used.
        // We map a 4KiB page and we don't use the `HUGE_PAGE` flag.
        memory::vmm::map_address(
            p.page_table,
            VirtAddr::new(segment.p_vaddr + mapped),
            page,
            flags,
        )
        .map_err(|_| OutOfMemory {})?;
        mapped += Size4KiB::SIZE;
    }

    Ok(())
}

/// Write a segment to the process' memory.
///
/// # Arguments
/// - `file_id` - The ELF file of the process.
/// - `p` - The process' struct.
/// - `segment` - The segment to write.
///
/// # Panics
/// Panic if the segment has not yet been mapped into the process' address space.
///
/// # Safety
/// This function is unsafe because it assumes the segment has been loaded to memory correctly.
unsafe fn write_segment(file_id: u64, p: &Process, segment: &ElfPhdr) {
    let mut address;
    let mut buffer;
    let mut to_write = segment.p_memsz;

    loop {
        // UNWRAP: The page table is not null and we
        // panic if the segment has not been mapped to memory.
        address = memory::vmm::virtual_to_physical(p.page_table, VirtAddr::new(segment.p_vaddr))
            .unwrap()
            .as_u64();
        buffer = core::slice::from_raw_parts_mut(
            (address + memory::HHDM_OFFSET) as *mut u8,
            core::cmp::min(to_write, Size4KiB::SIZE) as usize,
        );

        fs::read(file_id as usize, buffer, segment.p_offset as usize);

        if to_write <= Size4KiB::SIZE {
            return;
        }

        to_write -= Size4KiB::SIZE;
    }
}

/// Load a process' virtual address space.
///
/// # Arguments
/// - `file_id` - The ELF file to load.
///
/// # Returns
/// The function returns a newly created `Process` struct or an `OutOfMemory` error.
///
/// # Safety
/// This function is unsafe because it assumes that `file_id` points to a valid
/// ELF file.
pub unsafe fn load_process(file_id: u64) -> Result<Process, OutOfMemory> {
    let header = get_header(file_id);
    let stack_page = memory::page_allocator::allocate().ok_or(OutOfMemory {})?;
    let p = Process::new(header.e_entry, PROCESS_STACK_POINTER, false).ok_or(OutOfMemory {})?;

    for entry in &get_program_table(file_id, &header) {
        if entry.p_type == PT_LOAD {
            map_segment(&p, entry).map_err(|e| {
                super::terminate_process(&p);

                e
            })?;
            write_segment(file_id, &p, entry);
        }
    }
    // The page table is not null because we check it in `create_page_table`.
    // There are no problems with the huge page flag.
    // The file should not contains segments that will overlap with the process' stack.
    // Therefore, if there's an error we return `OutOfMemory`.
    memory::vmm::map_address(
        p.page_table,
        VirtAddr::new(PROCESS_STACK_POINTER - Size4KiB::SIZE),
        stack_page,
        PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE | PageTableFlags::WRITABLE,
    )
    .map_err(|_| OutOfMemory {})?;

    Ok(p)
}

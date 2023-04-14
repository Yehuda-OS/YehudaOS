use core::{
    alloc::{GlobalAlloc, Layout},
    mem::size_of,
};

use super::{Process, SchedulerError};
use crate::memory;
use crate::memory::allocator;
use alloc::vec::Vec;
use fs_rs::fs;
use x86_64::{
    registers::control::Cr3,
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
fn map_segment(p: &Process, segment: &ElfPhdr) -> Result<(), SchedulerError> {
    let flags =
        PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE | PageTableFlags::WRITABLE;
    let mut mapped = 0;
    let mut page;

    while mapped < segment.p_memsz {
        page = memory::page_allocator::allocate().ok_or(SchedulerError::OutOfMemory)?;
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
        .map_err(|_| SchedulerError::OutOfMemory)?;
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

/// Allocate memory in a process' heap.
///
/// # Arguments
/// - `p` - The process.
/// - `size` - The allocation size.
///
/// # Safety
/// Assumes the process' page tables are loaded.
///
/// # Returns
/// Returnes the allocation or `None` if the allocation failed.
unsafe fn alloc(p: &super::Process, size: usize) -> Option<*mut u8> {
    let layout = Layout::from_size_align(size, allocator::DEFAULT_ALIGNMENT);
    let mut allocation = core::ptr::null_mut();

    if let Ok(layout) = layout {
        allocation = p.allocator.alloc(layout);
    }

    if allocation.is_null() {
        None
    } else {
        Some(allocation)
    }
}

/// Write the commandline arguments to the process' heap.
///
/// # Arguments
/// - `p` - The process.
/// - `argv` - The arguments.
///
/// # Returns
/// A pointer to the `argv` array in the process' heap or an `OutOfMemory` error if the allocation
/// fails.
fn write_args(p: &super::Process, argv: &Vec<&str>) -> Result<*const *const u8, SchedulerError> {
    let cr3 = Cr3::read().0.start_address();
    let pointers_arr;
    let mut allocation;

    // SAFETY: The higher half should be the same for every page table.
    unsafe {
        memory::load_tables_to_cr3(p.page_table);
        pointers_arr = alloc(p, argv.len() * size_of::<u64>()).ok_or(SchedulerError::OutOfMemory)?
            as *mut *const u8;
    }
    for (i, arg) in argv.iter().enumerate() {
        // SAFETY: We loaded the process' page table and `arg` is an str so it should be
        // checked from before, and `allocation` was returned from
        // our allocator so it should be valid.
        unsafe {
            allocation = alloc(p, arg.len()).ok_or(SchedulerError::OutOfMemory)?;

            core::ptr::copy(arg.as_ptr(), allocation, arg.len());
            *pointers_arr.add(i) = allocation;
        }
    }
    // SAFETY: Load back the old page tables.
    unsafe { memory::load_tables_to_cr3(cr3) }

    Ok(pointers_arr)
}

impl super::Process {
    /// Load a process' virtual address space.
    ///
    /// # Arguments
    /// - `file_id` - The ELF file to load.
    /// - `cwd` - The current working directory for the new process.
    /// - `argv` - The commandline arguments for the process.
    ///
    /// # Returns
    /// The function returns a newly created `Process` struct or an `OutOfMemory` error.
    ///
    /// # Safety
    /// This function is unsafe because it assumes that `file_id` points to a valid
    /// ELF file.
    pub unsafe fn new_user_process(
        file_id: u64,
        cwd: usize,
        argv: &Vec<&str>,
    ) -> Result<Self, SchedulerError> {
        let header = get_header(file_id);
        let stack_page = memory::page_allocator::allocate().ok_or(SchedulerError::OutOfMemory)?;
        let page_table = super::create_page_table().ok_or(SchedulerError::OutOfMemory)?;
        let mut p = Process {
            registers: super::Registers::default(),
            stack_pointer: PROCESS_STACK_POINTER,
            page_table,
            instruction_pointer: header.e_entry,
            flags: super::INTERRUPT_FLAG_ON,
            pid: super::allocate_pid(),
            kernel_task: false,
            stack_start: VirtAddr::new(PROCESS_STACK_POINTER),
            cwd,
            allocator: allocator::Locked::new(allocator::Allocator::new(
                allocator::USER_HEAP_START,
                page_table,
            )),
        };

        p.registers.rdi = argv.len() as u64;
        p.registers.rsi = write_args(&p, argv)? as u64;

        for entry in &get_program_table(file_id, &header) {
            if entry.p_type == PT_LOAD {
                map_segment(&p, entry)?;
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
        .map_err(|_| SchedulerError::OutOfMemory)?;

        Ok(p)
    }
}

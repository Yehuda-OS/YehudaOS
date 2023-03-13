use super::memory;
use crate::{io, syscalls};
use alloc::vec::Vec;
use core::arch::asm;
use core::fmt;
use x86_64::{
    structures::paging::{PageSize, PhysFrame, Size4KiB},
    PhysAddr,
};
mod kernel_tasks;
mod loader;

static mut CURR_PROC: Option<Process> = None;
static mut PROC_QUEUE: Vec<(Process, u8)> = Vec::new();

const KERNEL_CODE_SEGMENT: u16 = super::gdt::KERNEL_CODE;
const KERNEL_DATA_SEGMENT: u16 = super::gdt::KERNEL_DATA;
const USER_CODE_SEGMENT: u16 = super::gdt::USER_CODE | 3;
const USER_DATA_SEGMENT: u16 = super::gdt::USER_DATA | 3;
const INTERRUPT_FLAG_ON: u64 = 0x200;

static mut TSS_ENTRY: TaskStateSegment = TaskStateSegment {
    reserved0: 0,
    rsp0: 0,
    rsp1: 0,
    rsp2: 0,
    reserved1: 0,
    ist1: 0,
    ist2: 0,
    ist3: 0,
    ist4: 0,
    ist5: 0,
    ist6: 0,
    ist7: 0,
    reserved2: 0,
    reserved3: 0,
    io_permission_bitmap: 0,
};

#[derive(Debug)]
pub enum SchedulerError {
    OutOfMemory,
}

impl fmt::Display for SchedulerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            SchedulerError::OutOfMemory => write!(f, "not enough memory to create a process"),
        }
    }
}

#[repr(packed)]
#[allow(unused)]
pub struct TaskStateSegment {
    reserved0: u32,
    rsp0: u64,
    rsp1: u64,
    rsp2: u64,
    reserved1: u64,
    ist1: u64,
    ist2: u64,
    ist3: u64,
    ist4: u64,
    ist5: u64,
    ist6: u64,
    ist7: u64,
    reserved2: u64,
    reserved3: u16,
    io_permission_bitmap: u16,
}

#[derive(Default, Debug, Clone, Copy)]
pub struct Registers {
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rbp: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
}

#[repr(u8)]
pub enum ProcessStates {
    New,
    Ready,
    Running,
    Waiting,
    Terminate,
}

#[repr(C)]
pub struct Process {
    pub registers: Registers,
    pub stack_pointer: u64,
    pub kernel_task: bool,
    pub page_table: PhysAddr,
    pub instruction_pointer: u64,
    pub flags: u64,
}

impl Drop for Process {
    fn drop(&mut self) {
        if self.kernel_task {
            kernel_tasks::deallocate_stack(self.stack_pointer);
        } else {
            memory::vmm::page_table_walker(self.page_table, &|virt, physical| {
                if virt.as_u64() < memory::HHDM_OFFSET {
                    memory::vmm::unmap_address(self.page_table, virt).unwrap();
                    unsafe {
                        memory::page_allocator::free(PhysFrame::from_start_address_unchecked(
                            physical,
                        ))
                    }
                }
            });
            // SAFETY: The page table has been created with `create_page_table`.
            unsafe {
                memory::page_allocator::free(PhysFrame::from_start_address_unchecked(
                    self.page_table,
                ))
            }
        }
    }
}

/// Get the `rsp0` field from the TSS.
pub fn get_kernel_stack() -> u64 {
    unsafe { TSS_ENTRY.rsp0 }
}

/// Returns a mutable reference to the currently running process.
///
/// # Safety
/// Should not be used in a multi-threaded situation.
pub unsafe fn get_running_process() -> &'static mut Option<Process> {
    &mut CURR_PROC
}

/// function that push process into the process queue
///
/// # Arguments
/// - `p` - the process
pub fn add_to_the_queue(p: Process) {
    let proc: (Process, u8) = if p.kernel_task {
        (p, 15) // if the procrss is kernel task it gets higher priority
    } else {
        (p, 0)
    };

    // SAFETY: The shceduler should not be referenced in a multithreaded situation.
    unsafe {
        PROC_QUEUE.push(proc);
        PROC_QUEUE.sort_unstable_by(|a, b| a.1.cmp(&b.1));
        for i in 0..PROC_QUEUE.len() {
            PROC_QUEUE[i].1 += 1;
        }
    }
}

/// Re-add the current process to the process queue and set the current process to `None`.
///
/// # Panics
/// Panics if there is no current process.
pub fn switch_current_process() {
    // SAFETY: `CURR_PROC` is a valid `Process` struct and the ownership of it is now moved to the
    // process queue, so no resources are leaked.
    unsafe {
        let curr = core::ptr::read(CURR_PROC.as_ref().unwrap());

        core::ptr::write(&mut CURR_PROC, None);
        add_to_the_queue(curr);
    }
}

/// Load a process from the queue.
///
/// # Panics
/// Panics if the process queue is empty.
pub unsafe fn load_from_queue() -> ! {
    let p = PROC_QUEUE.pop().unwrap();

    if let Some(process) = &CURR_PROC {
        add_to_the_queue(core::ptr::read(process))
    }
    core::ptr::write(&mut CURR_PROC, Some(p.0));
    load_context(CURR_PROC.as_ref().unwrap());
}

/// Returns the address of the Task State Segment.
pub fn get_tss_address() -> u64 {
    unsafe { &TSS_ENTRY as *const _ as u64 }
}

/// Load kernel's stack pointer to the TSS and load the
/// TSS segment selector to the task register.
///
/// # Safety
/// This function is unsafe because it requires a valid GDT with a TSS segment descriptor.
pub unsafe fn load_tss() {
    asm!("mov {0}, rsp", out(reg)TSS_ENTRY.rsp0);
    asm!("mov {0}, rsp", out(reg)TSS_ENTRY.ist1);
    asm!("ltr ax", in("ax")super::gdt::TSS);
}

/// Start running a user process in ring 3.
///
/// # Arguments
/// - `p` - The process' data structure.
///
/// # Safety
/// This function is unsafe because it jumps to a code at a specific
/// address and deletes the entire call stack.
pub unsafe fn load_context(p: &Process) -> ! {
    let (code_segment, data_segment) = if p.kernel_task {
        (KERNEL_CODE_SEGMENT, KERNEL_DATA_SEGMENT)
    } else {
        (USER_CODE_SEGMENT, USER_DATA_SEGMENT)
    };
    let p_address = p as *const Process as u64;

    memory::load_tables_to_cr3(p.page_table);
    // Write the address of the process to later use it in the syscall handler.
    io::wrmsr(syscalls::KERNEL_GS_BASE, p_address);
    // Move the user data segment selector to the segment registers and push
    // the future `ss`, `rsp`, `rflags`, `cs` and `rip` that will later be popped by `iretq`.
    asm!("
    swapgs
    mov ds, {0:x}
    mov es, {0:x}
    mov fs, {0:x}

    push {0:r}
    push {rsp}
    push {flags}
    push {1:r}
    push {rip}
    ",
        in(reg)data_segment, in(reg)code_segment,
        flags=in(reg)p.flags,
        rsp=in(reg)p.stack_pointer, rip=in(reg)p.instruction_pointer
    );
    // Push the future `rbx` and `rbp` to later pop them.
    asm!("
    push {rbx}
    push {rbp}
    ",
            rbx=in(reg)p.registers.rbx,
            rbp=in(reg)p.registers.rbp,
    );
    // Pop `rbx` and `rbp` that we pushed earlier and perform the return
    // after loading the general purpose register with the appropriate values.
    asm!("
    pop rbp
    pop rbx
    iretq",
        in("rax")p.registers.rax,
        in("rcx")p.registers.rcx,
        in("rdx")p.registers.rdx,
        in("rsi")p.registers.rsi,
        in("rdi")p.registers.rdi,
        in("r8")p.registers.r8,
        in("r9")p.registers.r9,
        in("r10")p.registers.r10,
        in("r11")p.registers.r11,
        in("r12")p.registers.r12,
        in("r13")p.registers.r13,
        in("r14")p.registers.r14,
        in("r15")p.registers.r15,
        options(noreturn)
    );
}

/// Create a page table for a process and copy the higher half of the kernel's page table to it
/// because the kernel's memory is at the higher half of the address space.
///
/// # Returns
/// The address of the new page table or `None` if there is no free space for a page table.
///
/// # Safety
/// A valid kernel's page table is required.
unsafe fn create_page_table() -> Option<PhysAddr> {
    let table = memory::vmm::create_page_table()?;
    let high_kernel_table = memory::get_page_table() + Size4KiB::SIZE / 2;
    let high_user_table = table + Size4KiB::SIZE / 2;

    core::ptr::copy_nonoverlapping(
        (high_kernel_table.as_u64() + memory::HHDM_OFFSET) as *const u8,
        (high_user_table.as_u64() + memory::HHDM_OFFSET) as *mut u8,
        Size4KiB::SIZE as usize / 2,
    );

    Some(table)
}
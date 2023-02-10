use super::memory;
use crate::mutex::{Mutex, MutexGuard};
use alloc::vec::Vec;
use core::arch::asm;
use core::fmt;
use lazy_static::lazy_static;
use x86_64::{
    structures::paging::{PageSize, Size4KiB},
    PhysAddr,
};
mod kernel_tasks;
mod loader;

lazy_static! {
    pub static ref PROC_QUEUE: Mutex<Vec<(Process, u8)>> = Mutex::new(Vec::new());
}

static CURR_PROC: Mutex<Option<Process>> = Mutex::new(None);

const KERNEL_CODE_SEGMENT: u16 = super::gdt::KERNEL_CODE;
const KERNEL_DATA_SEGMENT: u16 = super::gdt::KERNEL_DATA;
const USER_CODE_SEGMENT: u16 = super::gdt::USER_CODE | 3;
const USER_DATA_SEGMENT: u16 = super::gdt::USER_DATA | 3;

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

#[derive(Clone)]
pub struct Process {
    pub registers: Registers,
    pub page_table: PhysAddr,
    pub stack_pointer: u64,
    pub instruction_pointer: u64,
    pub flags: u64,
    pub kernel_task: bool,
}

/// Returns a mutable reference to the currently running process.
/// Should not be used in a multi-threaded situation.
pub fn get_running_process() -> MutexGuard<'static, Option<Process>> {
    CURR_PROC.lock()
}

/// function that push process into the process queue
///
/// # Arguments
/// - `p` - the process
pub fn add_to_the_queue(p: Process) {
    let mut proc_queue = PROC_QUEUE.lock();

    let proc: (Process, u8) = if p.kernel_task {
        (p, 15) // if the procrss is kernel task it gets higher priority
    } else {
        (p, 0)
    };

    proc_queue.push(proc);
    proc_queue.sort_unstable_by(|a, b| a.1.cmp(&b.1));
    for i in 0..proc_queue.len() {
        proc_queue[i].1 += 1;
    }
}

/// Load a process from the queue.
///
/// # Panics
/// Panics if the process queue is empty.
pub fn load_from_queue() -> ! {
    let mut proc_queue = PROC_QUEUE.lock();
    let p = proc_queue.pop().unwrap();
    let mut current_process = CURR_PROC.lock();

    if let Some(process) = &*current_process {
        unsafe { add_to_the_queue(core::ptr::read(process)) }
    }
    unsafe {
        core::ptr::write(&mut *current_process, Some(p.0));
        load_context(current_process.as_ref().unwrap());
    }
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
    asm!("ltr ax", in("ax")super::gdt::TSS);
}

/// Save all the general purpose registers of a process.
///
/// # Returns
/// A data structure with the saved values of the registers.
///
/// # Safety
/// This function is unsafe because it assumes the saved value of the `rbp` register is on
/// the top of the stack and the `rbp` register contains the pointer to it
/// (Happens with every function that uses a stack frame).
/// This is unsafe because the function reads the caller's stack.
#[inline(always)]
pub unsafe fn save_context() -> Registers {
    let mut registers: Registers;

    asm!(
        "
        push r15
        push r14
        push r13
        push r12
        push r11
        push r10
        push r9
        push r8
        mov r8, [rbp]
        push r8
        push rdi
        push rsi
        push rdx
        push rcx
        push rbx
        push rax
        "
    );
    registers = Registers::default();
    asm!(
        "
        pop {0}
        pop {1}
        pop {2}
        pop {3}
        pop {4}
        pop {5}
        pop {6}
        pop {7}
        pop {8}
        pop {9}
        pop {10}
        pop {11}
        pop {12}
        pop {13}
        ",
        out(reg)registers.rax,
        out(reg)registers.rbx,
        out(reg)registers.rcx,
        out(reg)registers.rdx,
        out(reg)registers.rsi,
        out(reg)registers.rdi,
        out(reg)registers.rbp,
        out(reg)registers.r8,
        out(reg)registers.r9,
        out(reg)registers.r10,
        out(reg)registers.r11,
        out(reg)registers.r12,
        out(reg)registers.r13,
        out(reg)registers.r14,
    );
    asm!(
        "
        pop {0}
        ",
        out(reg)registers.r15,
    );

    registers
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

    memory::load_tables_to_cr3(p.page_table);
    // Move the user data segment selector to the segment registers and push
    // the future `ss`, `rsp`, `rflags`, `cs` and `rip` that will later be popped by `iretq`.
    asm!("
    mov ds, {0:x}
    mov es, {0:x}
    mov fs, {0:x}
    mov gs, {0:x}

    push {0:r}
    push {rsp}
    pushfq
    push {1:r}
    push {rip}
    ",
        in(reg)data_segment, in(reg)code_segment,
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

fn terminate_process(p: &Process) {
    // TODO
}

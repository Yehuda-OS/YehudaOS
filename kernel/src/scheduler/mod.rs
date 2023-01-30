use super::memory;
use core::arch::asm;
use x86_64::{
    structures::paging::{PageSize, Size4KiB},
    PhysAddr,
};

mod loader;

const CODE_SEGMENT: u16 = super::gdt::USER_CODE | 3;
const DATA_SEGMENT: u16 = super::gdt::USER_DATA | 3;

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

#[derive(Default, Debug)]
pub struct Registers {
    rax: u64,
    rbx: u64,
    rcx: u64,
    rdx: u64,
    rsi: u64,
    rdi: u64,
    rbp: u64,
    r8: u64,
    r9: u64,
    r10: u64,
    r11: u64,
    r12: u64,
    r13: u64,
    r14: u64,
    r15: u64,
}

pub struct Process {
    registers: Registers,
    page_table: PhysAddr,
    stack_pointer: u64,
    instruction_pointer: u64,
    flags: u64,
}

impl Process {
    /// Returns a new `Process` struct and creates a page table for it, or `None` if there
    /// is no free space for a page table.
    ///
    /// # Safety
    /// A valid kernel's page table is required.
    pub unsafe fn new() -> Option<Self> {
        Some(Process {
            registers: Registers::default(),
            page_table: create_page_table()?,
            stack_pointer: 0,
            instruction_pointer: 0,
            flags: 0,
        })
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
        in(reg)DATA_SEGMENT, in(reg)CODE_SEGMENT,
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

    core::ptr::copy_nonoverlapping(
        (memory::PAGE_TABLE + Size4KiB::SIZE / 2).as_u64() as *const u8,
        table.as_u64() as *mut u8,
        Size4KiB::SIZE as usize / 2,
    );

    Some(table)
}

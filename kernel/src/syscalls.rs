use super::io;
use super::scheduler;

const EFER: u32 = 0xc0000080;
const STAR: u32 = 0xc0000081;
const LSTAR: u32 = 0xc0000082;

pub unsafe fn initialize() {
    let rip = handler as u64;
    let cs = u64::from(super::gdt::KERNEL_CODE) << 32;

    io::wrmsr(LSTAR, rip);
    io::wrmsr(STAR, cs);
    // Enable syscalls by setting the first bit of the EFER MSR
    io::wrmsr(EFER, 1);
}

pub unsafe fn handler() -> ! {
    let registers = scheduler::save_context();
    // TODO Change later to get the currently running process.
    let mut proc = scheduler::Process {
        registers,
        // After we change this to the running process the page table field will already be loaded.
        page_table: unsafe { super::memory::PAGE_TABLE },
        stack_pointer: 0,
        instruction_pointer: 0,
        flags: 0,
    };

    // The `syscall` instruction saves the instruction pointer in `rcx` and the cpu flags in `r11`.
    proc.instruction_pointer = proc.registers.rcx;
    proc.flags = proc.registers.r11;
    // `rbp` holds the value of the stack pointer after pushing the original `rbp`.
    core::arch::asm!("
    mov {0}, rbp
    sub {0}, 8
    ",
        out(reg)proc.stack_pointer,
    );
    crate::println!("A syscall occured");

    scheduler::load_context(&proc);
}

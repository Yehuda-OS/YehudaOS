use bitflags::bitflags;

const MAX_LIMIT: u32 = 0xfffff;

pub const KERNEL_CODE: u16 = 0x8;
pub const KERNEL_DATA: u16 = 0x10;

static mut GDT: [Entry; 6] = [
    Entry::zeros(),
    Entry::zeros(),
    Entry::zeros(),
    Entry::zeros(),
    Entry::zeros(),
    Entry::zeros(),
];


#[repr(packed)]
#[allow(unused)]
struct Entry {
    limit0: u16,
    base0: u16,
    base1: u8,
    access: AccessByte,
    limit1_flags: u8,
    base2: u8,
}

bitflags! {
    struct AccessByte: u8 {
        /// If set on a data segment, the segment will be writable and if it is set on a code
        /// segment the segment will be readable.
        const READABLE_WRITEABLE = 1 << 1;
        const EXECUTABLE = 1 << 3;
        /// If not set, the segment will be a system segment.
        const CODE_OR_DATA = 1 << 4;
        /// If set, the segment will be accessible from ring 3.
        const RING3 = 1 << 4 | 1 << 5;
        const PRESENT = 1 << 7;

        /// `Available 32-bit TSS` type of a system segment.
        const TYPE_TSS = 0x9;
    }

    struct Flags: u8 {
        /// Must be set in 64 bit code segments.
        const LONG_MODE = 1 << 1;
        /// This is set in data segment.
        const DEFAULT_SIZE = 1 << 2;
        /// If set, the limit is a count of 4KiB blocks instead of 1 byte blocks.
        const GRANULARITY_4KIB = 1 << 3;
    }
}

impl Entry {
    pub const fn new(base: u64, limit: u32, access: AccessByte, flags: Flags) -> Self {
        Entry {
            limit0: limit as u16,
            base0: base as u16,
            base1: (base >> 16) as u8,
            access,
            limit1_flags: (flags.bits << 4) | (limit >> 16) as u8,
            base2: (base >> 24) as u8,
        }
    }

    pub const fn zeros() -> Self {
        Entry {
            limit0: 0,
            base0: 0,
            base1: 0,
            access: AccessByte { bits: 0 },
            limit1_flags: 0,
            base2: 0,
        }
    }
}

/// Create the GDT with the required segments.
pub fn create() {
    unsafe {
        GDT = [
            // NULL descriptor.
            Entry::zeros(),
            // Kernel mode code segment.
            Entry::new(
                0,
                MAX_LIMIT,
                AccessByte::from_bits_truncate(
                    AccessByte::PRESENT.bits
                        | AccessByte::CODE_OR_DATA.bits
                        | AccessByte::EXECUTABLE.bits
                        | AccessByte::READABLE_WRITEABLE.bits,
                ),
                Flags::from_bits_truncate(Flags::GRANULARITY_4KIB.bits | Flags::LONG_MODE.bits),
            ),
            // Kernel mode data segment.
            Entry::new(
                0,
                MAX_LIMIT,
                AccessByte::from_bits_truncate(
                    AccessByte::PRESENT.bits
                        | AccessByte::CODE_OR_DATA.bits
                        | AccessByte::READABLE_WRITEABLE.bits,
                ),
                Flags::from_bits_truncate(Flags::GRANULARITY_4KIB.bits | Flags::DEFAULT_SIZE.bits),
            ),
            // User mode code segment.
            Entry::new(
                0,
                MAX_LIMIT,
                AccessByte::from_bits_truncate(
                    AccessByte::PRESENT.bits
                        | AccessByte::CODE_OR_DATA.bits
                        | AccessByte::EXECUTABLE.bits
                        | AccessByte::READABLE_WRITEABLE.bits
                        | AccessByte::RING3.bits,
                ),
                Flags::from_bits_truncate(Flags::GRANULARITY_4KIB.bits | Flags::LONG_MODE.bits),
            ),
            // User mode data segment.
            Entry::new(
                0,
                MAX_LIMIT,
                AccessByte::from_bits_truncate(
                    AccessByte::PRESENT.bits
                        | AccessByte::CODE_OR_DATA.bits
                        | AccessByte::READABLE_WRITEABLE.bits
                        | AccessByte::RING3.bits,
                ),
                Flags::from_bits_truncate(Flags::GRANULARITY_4KIB.bits | Flags::DEFAULT_SIZE.bits),
            ),
            // Task State Segment
            Entry::new(
                super::scheduler::get_tss_address(),
                core::mem::size_of::<super::scheduler::TaskStateSegment>() as u32,
                AccessByte::from_bits_truncate(
                    AccessByte::PRESENT.bits | AccessByte::TYPE_TSS.bits,
                ),
                Flags::empty(),
            ),
        ]
    }
}

/// Loads new values to the segment registers.
/// Performs a far return to update the `cs` register.
/// To perform the far return, the function pops the value of `rbp` that was pushed when the
/// stack frame was created and then pops the return address then and pushes the new value of the
/// `cs` register and pushes the return address and then performs the far return.
/// 
/// # Safety
/// This function is unsafe because loading new values to the segment registers requires
/// a valid GDT to be already loaded.
#[allow(unreachable_code)]
unsafe fn reload_segments() {
    core::arch::asm!("
    pop rbp
    pop rcx

    mov ds, dx
    mov es, dx
    mov fs, dx
    mov gs, dx
    mov ss, dx

    push ax
    push rcx
    retfq
    "
    , in("ax")KERNEL_CODE, in("dx")KERNEL_DATA);
    loop {};
}

/// Load the GDT to the GDTR and activate the GDT.
/// Put the appropriate segment selectors in the appropriate registers.
///
/// # Safety
/// This function is unsafe because it changes the segment registers.
pub unsafe fn activate() {
    let limit = core::mem::size_of_val(&GDT) as u16 - 1;
    let base = &GDT as *const _ as u64;
    let gdt_descriptor = &limit as *const _ as u64;

    crate::println!(
        "base: {:p}, limit: {:p}, descriptor: {:#x}\nbase: {:#x}, limit: {:#x}",
        &base,
        &limit,
        gdt_descriptor,
        base,
        limit,
    );

    core::arch::asm!("lgdt [{gdt_descriptor}]", gdt_descriptor=in(reg)gdt_descriptor);
    reload_segments();
    loop {}
}

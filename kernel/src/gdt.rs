use bitflags::bitflags;
use x86_64::VirtAddr;

const MAX_LIMIT: u32 = 0xfffff;

pub const KERNEL_CODE: u16 = 0x28;
pub const KERNEL_DATA: u16 = 0x30;
pub const TSS: u16 = 0x48;

static mut GDT: [Entry; 10] = [
    Entry::zeros(),
    Entry::zeros(),
    Entry::zeros(),
    Entry::zeros(),
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

#[repr(packed)]
#[allow(unused)]
struct GDTDescriptor {
    limit: u16,
    base: VirtAddr,
}

bitflags! {
    struct AccessByte: u8 {
        /// If set on a data segment, the segment will be writable and if it is set on a code
        /// segment the segment will be readable.
        const READABLE_WRITEABLE = 1 << 1;
        const CONFORMING = 1 << 2;
        const EXECUTABLE = 1 << 3;
        /// If not set, the segment will be a system segment.
        const CODE_OR_DATA = 1 << 4;
        /// If set, the segment will be accessible from ring 3.
        const RING3 = 1 << 6 | 1 << 5;
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

    #[allow(unused)]
    fn print(&self) {
        use crate::println;

        println!(
            "Base: {:#x}",
            self.base0 as u64 | ((self.base1 as u64) << 16) | ((self.base2 as u64) << 24)
        );
        println!(
            "Limit: {:#x}",
            self.limit0 as u32 | ((self.limit1_flags as u32 & 0xf) << 16)
        );
        println!("Access: {:?}", self.access);
        println!(
            "Flags: {:?}",
            Flags::from_bits_truncate(self.limit1_flags & 0xf0 >> 4)
        );
        println!("Entry bits: {:#x}", self.bits())
    }

    fn bits(&self) -> u64 {
        self.limit0 as u64
            | ((self.base0 as u64) << 16)
            | ((self.base1 as u64) << 32)
            | ((self.access.bits as u64) << 40)
            | ((self.limit1_flags as u64) << 48)
            | ((self.base2 as u64) << 56)
    }
}

/// Create the GDT with the required segments.
pub fn create() {
    // The 16 bit and 32 bit code and data segments are needed to use limine's terminal.
    unsafe {
        GDT = [
            // NULL descriptor.
            Entry::zeros(),
            // 16 bit code segment.
            Entry::new(
                0,
                0xffff,
                AccessByte::PRESENT
                    | AccessByte::CODE_OR_DATA
                    | AccessByte::EXECUTABLE
                    | AccessByte::READABLE_WRITEABLE,
                Flags::empty(),
            ),
            // 16 bit data segment.
            Entry::new(
                0,
                0xffff,
                AccessByte::PRESENT | AccessByte::CODE_OR_DATA | AccessByte::READABLE_WRITEABLE,
                Flags::empty(),
            ),
            // 32 bit code segment.
            Entry::new(
                0,
                MAX_LIMIT,
                AccessByte::PRESENT
                    | AccessByte::CODE_OR_DATA
                    | AccessByte::EXECUTABLE
                    | AccessByte::READABLE_WRITEABLE,
                Flags::GRANULARITY_4KIB | Flags::DEFAULT_SIZE,
            ),
            // 32 bit data segment
            Entry::new(
                0,
                MAX_LIMIT,
                AccessByte::PRESENT | AccessByte::CODE_OR_DATA | AccessByte::READABLE_WRITEABLE,
                Flags::GRANULARITY_4KIB | Flags::DEFAULT_SIZE,
            ),
            // Kernel mode code segment.
            Entry::new(
                0,
                MAX_LIMIT,
                AccessByte::PRESENT
                    | AccessByte::CODE_OR_DATA
                    | AccessByte::EXECUTABLE
                    | AccessByte::READABLE_WRITEABLE,
                Flags::GRANULARITY_4KIB | Flags::LONG_MODE,
            ),
            // Kernel mode data segment.
            Entry::new(
                0,
                MAX_LIMIT,
                AccessByte::PRESENT | AccessByte::CODE_OR_DATA | AccessByte::READABLE_WRITEABLE,
                Flags::GRANULARITY_4KIB | Flags::LONG_MODE,
            ),
            // User mode code segment.
            Entry::new(
                0,
                MAX_LIMIT,
                AccessByte::PRESENT
                    | AccessByte::CODE_OR_DATA
                    | AccessByte::EXECUTABLE
                    | AccessByte::READABLE_WRITEABLE
                    | AccessByte::RING3,
                Flags::GRANULARITY_4KIB | Flags::LONG_MODE,
            ),
            // User mode data segment.
            Entry::new(
                0,
                MAX_LIMIT,
                AccessByte::PRESENT
                    | AccessByte::CODE_OR_DATA
                    | AccessByte::READABLE_WRITEABLE
                    | AccessByte::RING3,
                Flags::GRANULARITY_4KIB | Flags::LONG_MODE,
            ),
            // Task State Segment
            Entry::new(
                super::scheduler::get_tss_address(),
                core::mem::size_of::<super::scheduler::TaskStateSegment>() as u32 - 1,
                AccessByte::PRESENT | AccessByte::TYPE_TSS,
                Flags::empty(),
            ),
        ]
    }
}

/// Loads new values to the segment registers.
/// Performs a far return to update the `cs` register.
///
/// # Safety
/// This function is unsafe because loading new values to the segment registers requires
/// a valid GDT to be already loaded.
unsafe fn reload_segments() {
    core::arch::asm!("
    push rax
    lea rax, [2f]
    push rax
    retfq

    2:
        mov ds, dx
        mov es, dx
        mov fs, dx
        mov gs, dx
        mov ss, dx
    "
    , in("rax")KERNEL_CODE, in("dx")KERNEL_DATA);
}

/// Load the GDT to the GDTR and activate the GDT.
/// Put the appropriate segment selectors in the appropriate registers.
///
/// # Safety
/// This function is unsafe because it changes the segment registers.
pub unsafe fn activate() {
    let gdt_descriptor = GDTDescriptor {
        limit: core::mem::size_of_val(&GDT) as u16 - 1,
        base: VirtAddr::new(&GDT as *const _ as u64),
    };

    core::arch::asm!("lgdt [{gdt_descriptor}]", gdt_descriptor=in(reg)(&gdt_descriptor as *const _ as u64));
    reload_segments();
}

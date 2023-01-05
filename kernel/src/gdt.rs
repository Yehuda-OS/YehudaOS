use bitflags::bitflags;

static mut GDT: [Entry; 6] = [
    Entry::zeros(),
    Entry::zeros(),
    Entry::zeros(),
    Entry::zeros(),
    Entry::zeros(),
    Entry::zeros(),
];

/// Create the GDT with the required segments.
pub fn create() {
    unsafe {
        GDT = [
            // NULL descriptor.
            Entry::zeros(),
            // Kernel mode code segment.
            Entry::new(
                0,
                0xfffff,
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
                0xfffff,
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
                0xfffff,
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
                0xfffff,
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

pub fn activate() {
    // TODO
}

#[repr(packed)]
struct Entry {
    limit0: u16,
    base0: u16,
    base1: u8,
    access: AccessByte,
    limit1_flags: u8,
    base2: u8,
    base3: u32,
    reserved: u32,
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
            // Take the upper 4 bits of the upper half of the limit.
            limit1_flags: flags.bits | ((limit >> 16) << 4) as u8,
            base2: (base >> 24) as u8,
            base3: (base >> 32) as u32,
            reserved: 0,
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
            base3: 0,
            reserved: 0,
        }
    }
}

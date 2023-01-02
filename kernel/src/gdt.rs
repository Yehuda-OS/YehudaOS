use bitflags::bitflags;

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
        const LONG_MODE = 1 << 1;
        /// If set, the limit is a count of 4KiB blocks instead of 1 byte blocks.
        const GRANULARITY = 1 << 3;
    }
}

impl Entry {
    pub const fn new(base: u64, limit: u32, access: AccessByte, flags: Flags) -> Self {
        Entry {
            limit0: limit as u16,
            base0: base as u16,
            base1: (base >> 16) as u8,
            access,
            limit1_flags: flags.bits | (limit >> 16) as u8,
            base2: (base >> 24) as u8,
            base3: (base >> 32) as u32,
            reserved: 0,
        }
    }
}

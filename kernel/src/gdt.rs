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
}

impl Entry {
    pub const fn new() -> Self {
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

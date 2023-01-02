#[repr(packed)]
struct Entry {
    limit0: u16,
    base0: u16,
    base1: u8,
    access: u8,
    limit1_flags: u8,
    base2: u8,
    base3: u32,
    reserved: u32,
}

impl Entry {
    pub const fn new() -> Self {
        Entry {
            limit0: 0,
            base0: 0,
            base1: 0,
            access: 0,
            limit1_flags: 0,
            base2: 0,
            base3: 0,
            reserved: 0,
        }
    }
}

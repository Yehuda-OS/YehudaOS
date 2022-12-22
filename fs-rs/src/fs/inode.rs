use super::DIRECT_POINTERS;

#[derive(Clone, Copy)]
pub struct Inode {
    pub id: usize,
    pub directory: bool,
    pub size: usize,
    pub addresses: [usize; DIRECT_POINTERS],
}

impl Inode {
    pub const fn new() -> Self {
        Inode {
            id: 0,
            directory: false,
            size: 0,
            addresses: [0; DIRECT_POINTERS],
        }
    }
}

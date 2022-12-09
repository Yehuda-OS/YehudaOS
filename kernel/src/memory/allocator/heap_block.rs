use core::ptr::null_mut;

pub struct HeapBlock {
    size: u64,
    prev: *mut HeapBlock,
}

impl HeapBlock {
    const FREE_BIT: u8 = 63;
    const HAS_NEXT_BIT: u8 = 62;

    pub const fn empty() -> Self {
        HeapBlock {
            size: 0,
            prev: null_mut(),
        }
    }

    pub const fn new(free: bool, has_next: bool, mut size: u64, prev: *mut HeapBlock) -> Self {
        if free {
            size |= 1 << HeapBlock::FREE_BIT;
        }
        if has_next {
            size |= 1 << HeapBlock::HAS_NEXT_BIT;
        }

        HeapBlock { size, prev }
    }

    /// Get the size of the block.
    pub fn size(&self) -> u64 {
        // The two top most bits are used as flags.
        self.size << 2 >> 2
    }

    /// Returns `true` if the block is free.
    pub fn free(&self) -> bool {
        // The top most bit of the size represents if the block is free.
        self.size >> HeapBlock::FREE_BIT == 1
    }

    pub fn set_free(&mut self, free: bool) {
        if free {
            self.size |= 1 << HeapBlock::FREE_BIT;
        } else {
            self.size &= !(1 << HeapBlock::FREE_BIT);
        }
    }

    // Returns `true` if the block is not the last in the linked list.
    pub fn has_next(&self) -> bool {
        // The second top most bit of the size represents
        // whether the block has another block after it.
        self.size & (1 << HeapBlock::HAS_NEXT_BIT) >> HeapBlock::HAS_NEXT_BIT == 1
    }

    pub fn set_has_next(&mut self, has_next: bool) {
        if has_next {
            self.size |= 1 << HeapBlock::HAS_NEXT_BIT;
        } else {
            self.size &= !(1 << HeapBlock::HAS_NEXT_BIT);
        }
    }

    /// Returns `true` if the block is the first in the list.
    pub fn has_prev(&self) -> bool {
        self.prev == null_mut()
    }

    // Get the next heap block in the list.
    pub fn next(&self) -> *mut HeapBlock {
        if self.has_next() {
            unsafe {
                let start_of_block = (self as *const HeapBlock).offset(1) as u64;

                (start_of_block + self.size) as *mut HeapBlock
            }
        } else {
            null_mut()
        }
    }

    pub fn prev(&self) -> *mut HeapBlock {
        self.prev
    }
}

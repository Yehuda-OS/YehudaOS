use super::blkdev;
use super::BLOCK_SIZE;

pub const DIRECT_POINTERS: usize = 12;
const POINTER_SIZE: usize = core::mem::size_of::<usize>();
pub const MAX_FILE_SIZE: usize = DIRECT_POINTERS * BLOCK_SIZE + BLOCK_SIZE / POINTER_SIZE * BLOCK_SIZE;

#[derive(Clone, Copy)]
pub struct Inode {
    pub id: usize,
    pub directory: bool,
    size: usize,
    pub addresses: [usize; DIRECT_POINTERS],
    pub indirect_pointer: usize,
}

impl Inode {
    pub const fn new() -> Self {
        Inode {
            id: 0,
            directory: false,
            size: 0,
            addresses: [0; DIRECT_POINTERS],
            indirect_pointer: 0,
        }
    }

    pub fn size(&self) -> usize {
        self.size
    }

    /// Set the size of an inode to `value`.
    /// 
    /// # Returns
    /// Returns a `MaximumSizeExceeded` error if the new size exceeds the maximum file size.
    pub fn set_size(&mut self, value: usize) -> Result<(), super::FsError> {
        if value > MAX_FILE_SIZE {
            return Err(super::FsError::MaximumSizeExceeded)
        }

        self.size = value;

        Ok(())
    }

    /// Returns the `index`th pointer of the inode or `Err` if the `index` exceeds the maximum
    /// file size divided by the block size.
    ///
    /// # Arguments
    /// - `index` - The index of the pointer.
    pub fn get_ptr(&self, index: usize) -> Result<usize, ()> {
        let offset;
        let mut ptr: usize = 0;

        if index < DIRECT_POINTERS {
            return Ok(self.addresses[index]);
        }

        offset = (index - DIRECT_POINTERS) * POINTER_SIZE;
        if offset > BLOCK_SIZE {
            return Err(());
        }
        unsafe {
            blkdev::read(
                self.indirect_pointer + offset,
                POINTER_SIZE,
                &mut ptr as *mut _ as *mut u8,
            );
        }

        Ok(ptr)
    }

    /// Set the value of the `index`th pointer.
    ///
    /// # Arguments
    /// - `index` - The index of the pointer.
    /// - `value` - The value to change to.
    ///
    /// # Returns
    /// `Err` if the pointer exceeds the maximum file size
    /// divided by the block size and `Ok` otherwise.
    pub fn set_ptr(&mut self, index: usize, value: usize) -> Result<(), ()> {
        let offset;

        if index < DIRECT_POINTERS {
            self.addresses[index] = value;

            return Ok(());
        }

        offset = (index - DIRECT_POINTERS) * POINTER_SIZE;
        if offset > BLOCK_SIZE {
            return Err(());
        }
        if self.indirect_pointer == 0 {
            if let Some(indirect_pointer) = super::allocate_block() {
                self.indirect_pointer = indirect_pointer;
            } else {
                return Err(());
            }
        }
        unsafe {
            blkdev::write(
                self.indirect_pointer + offset,
                POINTER_SIZE,
                &value as *const _ as *const u8,
            );
        };

        Ok(())
    }
}

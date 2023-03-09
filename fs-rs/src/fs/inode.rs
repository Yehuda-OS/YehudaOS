use super::blkdev;
use super::FsError;
use super::BLOCK_SIZE;

pub const DIRECT_POINTERS: usize = 12;
const POINTER_SIZE: usize = core::mem::size_of::<usize>();
const POINTERS_PER_BLOCK: usize = BLOCK_SIZE / POINTER_SIZE;
pub const MAX_FILE_SIZE: usize =
    BLOCK_SIZE * (DIRECT_POINTERS + POINTERS_PER_BLOCK * (POINTERS_PER_BLOCK + 1));

#[derive(Clone, Copy, Debug, Default)]
pub struct Inode {
    id: usize,
    directory: bool,
    size: usize,
    addresses: [usize; DIRECT_POINTERS],
    indirect_pointer: usize,
    double_indirect_pointer: usize,
}

impl Inode {
    pub fn is_dir(&self) -> bool {
        self.directory
    }

    pub fn set_as_dir(&mut self, value: bool) {
        self.directory = value;
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn set_id(&mut self, value: usize) {
        self.id = value;
    }

    pub fn size(&self) -> usize {
        self.size
    }

    /// Sets the size of an inode to `value`.
    /// Deallocates the unused pointers, it is the responsible of the caller to prevent
    /// any dangling pointers.
    ///
    /// # Returns
    /// Returns a `MaximumSizeExceeded` error if the new size exceeds the maximum file size.
    pub fn set_size(&mut self, value: usize) -> Result<(), FsError> {
        if value > MAX_FILE_SIZE {
            return Err(super::FsError::MaximumSizeExceeded);
        }

        if value / BLOCK_SIZE <= DIRECT_POINTERS && self.indirect_pointer != 0 {
            super::deallocate_block(self.indirect_pointer);
            self.indirect_pointer = 0;
        }

        self.size = value;

        Ok(())
    }

    /// Returns the `index`th pointer of the inode or `MaximumSizeExceeded` if the `index`
    /// exceeds the maximum file size divided by the pointer size.
    ///
    /// # Arguments
    /// - `index` - The index of the pointer.
    pub fn get_ptr(&self, mut index: usize) -> Result<usize, FsError> {
        let mut offset;
        let mut ptr = 0;

        if index < DIRECT_POINTERS {
            return Ok(self.addresses[index]);
        } else if index * BLOCK_SIZE >= MAX_FILE_SIZE {
            return Err(FsError::MaximumSizeExceeded);
        }

        index -= DIRECT_POINTERS;
        if index < POINTERS_PER_BLOCK {
            offset = index * POINTER_SIZE;

            if self.indirect_pointer == 0 {
                ptr = 0;
            } else {
                unsafe {
                    blkdev::read(
                        self.indirect_pointer + offset,
                        POINTER_SIZE,
                        &mut ptr as *mut _ as *mut u8,
                    )
                }
            }
        } else {
            index -= POINTERS_PER_BLOCK;
            offset = index / POINTERS_PER_BLOCK * POINTER_SIZE;

            if self.double_indirect_pointer == 0 {
                ptr = 0;
            } else {
                unsafe {
                    blkdev::read(
                        self.double_indirect_pointer + offset,
                        POINTER_SIZE,
                        &mut ptr as *mut _ as *mut u8,
                    )
                }
                index %= POINTERS_PER_BLOCK;
                offset = index * POINTER_SIZE;
                if ptr != 0 {
                    unsafe {
                        blkdev::read(ptr + offset, POINTER_SIZE, &mut ptr as *mut _ as *mut u8)
                    }
                }
            }
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
    /// - `MaximumSizeExceeded` if the pointer exceeds the file's size.
    /// - `NotEnoughDiskSpace` if there is no free space for the pointer.
    /// - `Ok` otherwise.
    pub fn set_ptr(&mut self, mut index: usize, value: usize) -> Result<(), FsError> {
        let mut offset;
        let mut ptr = 0;

        if index * BLOCK_SIZE > self.size {
            return Err(FsError::MaximumSizeExceeded);
        }

        if index < DIRECT_POINTERS {
            self.addresses[index] = value;

            return Ok(());
        }

        index -= DIRECT_POINTERS;
        if index < POINTERS_PER_BLOCK {
            offset = index * POINTER_SIZE;

            if self.indirect_pointer == 0 {
                self.indirect_pointer =
                    super::allocate_block().ok_or(FsError::NotEnoughDiskSpace)?;
                // SAFETY: We checked that the allocation succeeded.
                unsafe { blkdev::set(self.indirect_pointer, BLOCK_SIZE, 0) }
            }
            unsafe {
                blkdev::write(
                    self.indirect_pointer + offset,
                    POINTER_SIZE,
                    &value as *const _ as *const u8,
                )
            }
        } else {
            index -= POINTERS_PER_BLOCK;
            offset = index / POINTERS_PER_BLOCK * POINTER_SIZE;

            if self.double_indirect_pointer == 0 {
                self.double_indirect_pointer =
                    super::allocate_block().ok_or(FsError::NotEnoughDiskSpace)?;
                // SAFETY: We checked that the allocation succeeded.
                unsafe { blkdev::set(self.double_indirect_pointer, BLOCK_SIZE, 0) }
            }
            unsafe {
                blkdev::read(
                    self.double_indirect_pointer + offset,
                    POINTER_SIZE,
                    &mut ptr as *mut _ as *mut u8,
                )
            }
            if ptr == 0 {
                ptr = super::allocate_block().ok_or(FsError::NotEnoughDiskSpace)?;

                unsafe {
                    blkdev::write(
                        self.double_indirect_pointer + offset,
                        POINTER_SIZE,
                        &mut ptr as *mut _ as *mut u8,
                    )
                }
            }
            index %= POINTERS_PER_BLOCK;
            offset = index * POINTER_SIZE;
            unsafe {
                blkdev::write(ptr + offset, POINTER_SIZE, &value as *const _ as *const u8);
            }
        }

        Ok(())
    }
}

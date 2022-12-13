pub mod blkdev;

extern crate alloc;

use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use blkdev::BlkDev;
use core::result::{Result, Result::Err, Result::Ok};

use core::option::Option::None;
pub struct Fs {
    blkdev: BlkDev,
    disk_parts: DiskParts,
}

struct Header {
    magic: [u8; 4],
    version: u8,
}

struct DiskParts {
    block_bit_map: usize,
    inode_bit_map: usize,
    root: usize,
    unused: usize,
    data: usize,
}

const DIRECT_POINTERS: usize = 12;
const FILE_NAME_LEN: usize = 11;
const BLOCK_SIZE: usize = 16;
const BITS_IN_BYTE: usize = 8;
const BYTES_PER_INODE: usize = 16 * 1024;

#[derive(Clone, Copy)]
struct Inode {
    id: usize,
    directory: bool,
    size: usize,
    addresses: [usize; DIRECT_POINTERS],
}

struct DirEntry {
    name: String,
    id: usize,
}

/// private functions implementation
impl Fs {
    fn get_root_dir(&self) -> Inode {
        let ans: Inode = Inode {
            id: 0,
            directory: false,
            size: 0,
            addresses: [0; DIRECT_POINTERS],
        };

        unsafe {
            self.blkdev.read(
                self.disk_parts.root,
                core::mem::size_of::<Inode>(),
                &ans as *const _ as *mut u8,
            )
        };

        ans
    }

    fn get_inode(&self, mut path: String) -> Inode {
        let mut next_delimiter = path.find('/');
        let mut next_folder = String::new();
        let mut inode = self.get_root_dir();
        let mut dir_content: Vec<DirEntry> = Vec::new();
        let mut index: usize = 0;

        if path == "/" {
            return inode;
        }

        while next_delimiter != None {
            dir_content = self
                .read_inode_data(&inode)
                .expect("Error: failed to read the inode data");
            path = path[(next_delimiter.unwrap() + 1)..].to_string();
            next_delimiter = path.find('/');
            next_folder = path[0..next_delimiter.unwrap()].to_string();
            while dir_content[index as usize].name != next_folder {
                index += 1;
            }

            unsafe {
                self.blkdev.read(
                    self.get_inode_address(dir_content[index as usize].id),
                    core::mem::size_of::<Inode>(),
                    &mut inode as *mut Inode as *mut u8,
                )
            };
            index = 0;
        }

        inode
    }

    fn get_inode_address(&self, id: usize) -> usize {
        self.disk_parts.root + id * core::mem::size_of::<Inode>()
    }

    fn read_inode_data(&self, inode: &Inode) -> Result<Vec<DirEntry>, &'static str> {
        let last_pointer: usize = inode.size / BLOCK_SIZE;
        let mut pointer = 0;
        let mut to_read = 0;
        let mut buffer = Vec::<DirEntry>::new();

        while buffer.len() != inode.size {
            to_read = if pointer == last_pointer {
                inode.size % BLOCK_SIZE
            } else {
                BLOCK_SIZE
            };
            unsafe {
                self.blkdev.read(
                    inode.addresses[pointer],
                    to_read,
                    buffer.as_ptr() as *mut u8,
                )
            };
            pointer += 1;
        }

        Ok(buffer)
    }

    fn write_inode(&self, inode: &Inode) {
        unsafe {
            self.blkdev.write(
                self.get_inode_address(inode.id),
                core::mem::size_of::<Inode>(),
                inode as *const _ as *mut u8,
            )
        };
    }

    fn allocate_inode(&self) -> usize {
        self.allocate(self.disk_parts.inode_bit_map)
    }

    fn allocate(&self, bitmap_start: usize) -> usize {
        const BITS_IN_BUFFER: usize = 64;
        const BYTES_IN_BUFFER: usize = BITS_IN_BUFFER / BITS_IN_BYTE;
        const ALL_OCCUPIED: usize = 0xFFFFFFFFFFFFFFFF;
        let mut buffer: usize = 0;
        let mut address: usize = bitmap_start;

        // read the bitmap until an unoccupied memory is found
        loop {
            unsafe {
                self.blkdev
                    .read(address, BYTES_IN_BUFFER, buffer as *mut u8)
            };
            address += BYTES_IN_BUFFER;
            if buffer != ALL_OCCUPIED {
                break;
            }
        }
        address -= BYTES_IN_BUFFER;

        // read the buffer until an unoccupied memory is found
        for i in 0..BITS_IN_BUFFER {
            if buffer & (1 << i) == 0 {
                // if the (i)'s bit is 0
                buffer ^= 1 << i; // flip the bit to mark as occupied
                unsafe {
                    self.blkdev
                        .write(address, BYTES_IN_BUFFER, buffer as *mut u8);
                }
                // get the index in the bitmap
                address -= bitmap_start;
                address += i;

                // once we found unoccupied space, we finished our task
                break;
            }
        }
        address
    }

    fn deallocate(&self, bitmap_start: usize, n: usize) {
        let byte_address: usize = bitmap_start + n / BITS_IN_BYTE;
        let mut byte: usize = 0;
        let mut offset: usize = n % BITS_IN_BYTE;

        unsafe {
            self.blkdev
                .read(byte_address, 1, byte as *mut usize as *mut u8)
        };
        byte ^= 1 << offset; // flip the bit to mark as unoccupied
        unsafe {
            self.blkdev
                .write(byte_address, 1, byte as *mut usize as *mut u8)
        };
    }

    fn reallocate_blocks(&self, inode: &Inode, new_size: usize) -> Result<Inode, &'static str> {
        let mut used_blocks: isize = 0;
        let required_blocks: usize = new_size / BLOCK_SIZE + (new_size % BLOCK_SIZE != 0) as usize;
        let mut blocks_to_allocate: isize = 0;
        let mut new_addresses: Inode = *inode;

        if required_blocks > DIRECT_POINTERS {
            return Err("too many blocks are required");
        }

        while new_addresses.addresses[used_blocks as usize] != 0 {
            used_blocks += 1;
        }

        blocks_to_allocate = (required_blocks as isize - used_blocks) as isize;

        if blocks_to_allocate > 0 {
            for i in 0..blocks_to_allocate {
                new_addresses.addresses[used_blocks as usize] = self.allocate_block();
                used_blocks += 1;
            }
        } else if blocks_to_allocate < 0 {
            for i in 0..(-blocks_to_allocate) {
                used_blocks -= 1;
                self.deallocate_block(new_addresses.addresses[used_blocks as usize]);
                new_addresses.addresses[used_blocks as usize] = 0;
            }
        }

        Ok(new_addresses)
    }

    fn allocate_block(&self) -> usize {
        let mut address: usize = self.allocate(self.disk_parts.block_bit_map);

        // get physical address of the occupied block
        address *= BLOCK_SIZE;
        address += self.disk_parts.data;

        address
    }

    fn deallocate_block(&self, address: usize) {
        let block_number: usize = (address - self.disk_parts.data) / BLOCK_SIZE;

        self.deallocate(self.disk_parts.block_bit_map, block_number);
    }

    /*

    _addFileToFolder(const MyFs::DirEntry& file, MyFs::Inode& folder)
    {



        this->_writeInode(folder);
    }

    */

    fn add_file_to_folder(&self, file: &DirEntry, folder: &mut Inode) {
        let mut pointer: usize = folder.size / BLOCK_SIZE;
        let mut bytes_left: usize = core::mem::size_of_val(file);
        let mut address: isize = 0;
        let mut space_taken_in_last_block: isize = (folder.size % BLOCK_SIZE) as isize;
        let mut empty_space: isize = 0;
        let mut to_write: usize = 0;
        let mut written: usize = 0;

        *folder = self
            .reallocate_blocks(folder, folder.size + core::mem::size_of_val(file))
            .unwrap();

        address = folder.addresses[pointer] as isize + space_taken_in_last_block;
        empty_space = BLOCK_SIZE as isize - space_taken_in_last_block;
        to_write = if bytes_left > empty_space as usize {
            empty_space as usize
        } else {
            bytes_left
        };

        loop {
            unsafe {
                self.blkdev.write(
                    address as usize,
                    to_write as usize,
                    (file as *const _ as *mut u8).add(written),
                )
            };

            folder.size += to_write;
            written += to_write;
            bytes_left -= to_write;
            pointer += 1;
            address = folder.addresses[pointer] as isize;

            to_write = if bytes_left > BLOCK_SIZE {
                BLOCK_SIZE
            } else {
                bytes_left
            };

            if bytes_left == 0 {
                break;
            }
        }
    }
}

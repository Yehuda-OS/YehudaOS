mod blkdev;

extern crate alloc;

use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use blkdev::BlkDev;
use core::result::{Result, Result::Ok};

use core::option::Option::None;
struct Fs {
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

    fn get_inode(&self, path: String) -> Inode {
        let next_delimiter = path.find('/');
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
        let LAST_POINTER = inode.size / BLOCK_SIZE;
        let mut pointer = 0;
        let mut to_read = 0;
        let mut buffer = Vec::<DirEntry>::new();

        while buffer.len() != inode.size {
            to_read = if pointer == LAST_POINTER {
                inode.size % BLOCK_SIZE
            } else {
                BLOCK_SIZE
            };
            unsafe {
                self.blkdev
                    .read(inode.addresses[pointer], to_read, buffer.as_mut_ptr())
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
}

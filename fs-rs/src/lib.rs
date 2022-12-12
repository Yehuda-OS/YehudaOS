//#![no_std]
#![feature(strict_provenance)]
#![feature(inherent_associated_types)]

extern crate alloc;
use alloc::string::String;

const FS_NAME: &str = "myfs";

const LIST_CMD: &str = "ls";
const CONTENT_CMD: &str = "cat";
const CREATE_FILE_CMD: &str = "touch";
const CREATE_DIR_CMD: &str = "mkdir";
const EDIT_CMD: &str = "edit";
const TREE_CMD: &str = "tree";
const HELP_CMD: &str = "help";
const EXIT_CMD: &str = "exit";

static mut HELP_STRING: String = String::new();

mod fs;

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        unsafe {
            HELP_STRING = format!(
                "{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}",
                "The following commands are supported: \n".to_owned(),
                LIST_CMD,
                " [<directory>] - list directory content. \n",
                CONTENT_CMD,
                " <path> - show file content. \n",
                CREATE_FILE_CMD,
                " <path> - create empty file. \n",
                CREATE_DIR_CMD,
                " <path> - create empty directory. \n",
                EDIT_CMD,
                " <path> - re-set file content. \n",
                HELP_CMD,
                " - show this help messege. \n",
                EXIT_CMD,
                " - gracefully exit. \n"
            )
        };
        use alloc::vec::Vec;
        // let result = add(2, 2);
        // assert_eq!(result, 4);

        let blkdev = fs::blkdev::BlkDev::new(Vec::<u8>::new());
        let mut fs = fs::Fs::new(blkdev);

        fs.create_file("aaaa", false);
    }
}

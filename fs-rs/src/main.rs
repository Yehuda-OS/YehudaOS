#![feature(strict_provenance)]

use std::vec::Vec;

use fs::Fs;

const FS_NAME: &str = "fs";

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

fn recursive_print(fs: &mut Fs, path: String, prefix: String) {
    let dlist = fs.list_dir(&path);
    for (i, curr_entry) in dlist.iter().enumerate() {
        let entry_prefix = if i == dlist.len() - 1 {
            format!("{}└── ", prefix)
        } else {
            format!("{}├── ", prefix)
        };
        println!("{}{}", entry_prefix, curr_entry.name);

        if curr_entry.is_dir {
            let dir_prefix = if i == dlist.len() - 1 {
                format!("{}    ", prefix)
            } else {
                format!("{}│   ", prefix)
            };
            recursive_print(fs, format!("{}/{}", path, curr_entry.name), dir_prefix);
        }
    }
}

fn main() {
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
    // let result = add(2, 2);
    // assert_eq!(result, 4);

    let blkdev = fs::blkdev::BlkDev::new();
    let mut fs = fs::Fs::new(blkdev);

    // Declare the `FS_NAME` and `EXIT_CMD` constants
    const FS_NAME: &str = "fs";
    const EXIT_CMD: &str = "exit";

    // Declare `exit` as a mutable boolean
    let mut exit = false;

    // Start the main loop
    while !exit {
        println!("{}$ ", FS_NAME);
        // Read a command line from the standard input
        let mut cmdline = String::new();
        std::io::stdin().read_line(&mut cmdline).unwrap();

        // Skip empty command lines
        if cmdline == "" {
            continue;
        }

        // Split the command line into individual words
        let cmd: Vec<&str> = cmdline.split_whitespace().collect();

        // Handle the different possible commands
        match cmd[0] {
            // If the `list` command was entered, print the directory listing
            LIST_CMD => {
                let dlist = if cmd.len() == 1 {
                    fs.list_dir(&"/".to_string())
                } else if cmd.len() == 2 {
                    fs.list_dir(&cmd[1].to_string())
                } else {
                    println!("{}: one or zero arguments requested", LIST_CMD);
                    continue;
                };

                for i in 0..dlist.len() {
                    println!(
                        "{:15}{:10}",
                        dlist[i].name.clone() + (if dlist[i].is_dir { "/" } else { "" }),
                        dlist[i].file_size
                    );
                }
            }

            HELP_CMD => println!("{}", unsafe { HELP_STRING.clone() }),

            CREATE_FILE_CMD => {
                if cmd.len() == 2 {
                    fs.create_file(cmd[1].to_string(), false);
                } else {
                    println!("{}{}", CREATE_FILE_CMD, ": file path requested")
                }
            }

            CONTENT_CMD => {
                if cmd.len() == 2 {
                    println!("{}", fs.get_content(&cmd[1].to_string()));
                } else {
                    println!("{}{}", CONTENT_CMD, ": file path requested")
                }
            }

            TREE_CMD => recursive_print(&mut fs, "".to_string(), "".to_string()),

            EDIT_CMD => {
                if cmd.len() == 2 {
                    println!("Enter new file content");
                    let mut content: String = String::new();
                    let mut curr_line: String = String::new();
                    std::io::stdin().read_line(&mut curr_line);
                    loop {
                        content.push_str(&format!("{}\n", curr_line));
                        std::io::stdin().read_line(&mut curr_line);

                        if !curr_line.is_empty() {
                            break;
                        }
                    }
                    fs.set_content(&cmd[1].to_string(), &content);
                } else {
                    println!("{}{}", EDIT_CMD, ": file path requested");
                }
            }

            CREATE_DIR_CMD => {
                if cmd.len() == 2 {
                    fs.create_file((&cmd[1]).to_string(), true);
                } else {
                    println!("{}{}", CREATE_DIR_CMD, ": one argument requested");
                }
            }

            // If the `exit` command was entered, set the `exit` variable to true
            // to exit the main loop
            EXIT_CMD => exit = true,

            _ => println!("Unknown command"),
        }
    }
}

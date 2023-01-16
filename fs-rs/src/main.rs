#![feature(strict_provenance)]

use std::vec::Vec;

const FS_NAME: &str = "fs";

const LIST_CMD: &str = "ls";
const CONTENT_CMD: &str = "cat";
const CREATE_FILE_CMD: &str = "touch";
const CREATE_DIR_CMD: &str = "mkdir";
const EDIT_CMD: &str = "edit";
const TREE_CMD: &str = "tree";
const HELP_CMD: &str = "help";
const EXIT_CMD: &str = "exit";
const REMOVE_FILE_CMD: &str = "rm";
const REMOVE_DIR_CMD: &str = "rmdir";

static mut HELP_STRING: String = String::new();

mod fs;

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
    // Declare the `FS_NAME` and `EXIT_CMD` constants
    const FS_NAME: &str = "fs";
    const EXIT_CMD: &str = "exit";

    // Declare `exit` as a mutable boolean
    let mut exit = false;

    fs::init();
    // Start the main loop
    while !exit {
        println!("{}$ ", FS_NAME);
        // Read a command line from the standard input
        let mut cmdline = String::new();
        std::io::stdin().read_line(&mut cmdline).unwrap();

        // Skip empty command lines
        if cmdline.trim().is_empty() {
            continue;
        }

        // Split the command line into individual words
        let cmd: Vec<&str> = cmdline.split_whitespace().collect();

        // Handle the different possible commands
        match cmd[0] {
            // If the `list` command was entered, print the directory listing
            LIST_CMD => {
                let dlist = if cmd.len() == 1 {
                    fs::list_dir(&"/".to_string())
                } else if cmd.len() == 2 {
                    fs::list_dir(&cmd[1].to_string())
                } else {
                    println!("{}: one or zero arguments requested", LIST_CMD);
                    continue;
                };

                for i in 0..dlist.len() {
                    println!(
                        "{:15}{:10}",
                        dlist[i].name.clone().to_string()
                            + (if dlist[i].is_dir { "/" } else { "" }),
                        dlist[i].file_size
                    );
                }
            }

            HELP_CMD => println!("{}", unsafe { HELP_STRING.clone() }),

            CREATE_FILE_CMD => {
                if cmd.len() == 2 {
                    if let Err(e) = fs::create_file(cmd[1].to_string(), false) {
                        println!("{}", e);
                    }
                } else {
                    println!("{}{}", CREATE_FILE_CMD, ": file path requested")
                }
            }

            CONTENT_CMD => {
                if cmd.len() == 2 {
                    println!(
                        "{}",
                        fs::get_content(&cmd[1].to_string()).unwrap_or("".to_string())
                    );
                } else {
                    println!("{}{}", CONTENT_CMD, ": file path requested")
                }
            }

            EDIT_CMD => {
                if cmd.len() == 2 {
                    println!("Enter new file content");
                    let mut content: String = String::new();
                    let mut curr_line: String = String::new();
                    loop {
                        std::io::stdin()
                            .read_line(&mut curr_line)
                            .expect("failed to get input");
                        content.push_str(&format!("{}", curr_line));

                        if curr_line.trim().is_empty() {
                            break;
                        }

                        curr_line.clear();
                    }
                    if let Err(e) = fs::set_content(&cmd[1].to_string(), &mut content) {
                        println!("{}", e);
                    }
                } else {
                    println!("{}{}", EDIT_CMD, ": file path requested");
                }
            }

            CREATE_DIR_CMD => {
                if cmd.len() == 2 {
                    fs::create_file((&cmd[1]).to_string(), true);
                } else {
                    println!("{}{}", CREATE_DIR_CMD, ": one argument requested");
                }
            }

            REMOVE_FILE_CMD => {
                if cmd.len() == 2 {
                    if let Err(e) = fs::remove_file((&cmd[1]).to_string(), false) {
                        println!("{}", e);
                    }
                } else {
                    println!("{}{}", CREATE_DIR_CMD, ": one argument requested");
                }
            }

            REMOVE_DIR_CMD => {
                if cmd.len() == 2 {
                    if let Err(e) = fs::remove_file((&cmd[1]).to_string(), true) {
                        println!("{}", e);
                    }
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

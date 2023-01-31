use alloc::string::String;
use spin::Mutex;

pub static STDIN: Mutex<String> = Mutex::new(String::new());

const BACKSPACE: char = '\x08';

/// function to handle the keys that entered
///
/// # Arguments
/// - `ch` - the char to handle
pub fn key_handle(ch: char) {
    let mut stdin = STDIN.lock();

    if ch == BACKSPACE {
        stdin.pop();
        // have to implement function that deletes the char
    } else {
        stdin.push(ch);
    }
}

/// function that reads line and returns it
///
/// # Returns
/// the line it read
pub fn read_line() -> String {
    loop {
        let res = x86_64::instructions::interrupts::without_interrupts(|| {
            let mut stdin = STDIN.lock();
            match stdin.chars().next_back() {
                Some('\n') => {
                    let line = stdin.clone();
                    stdin.clear();
                    Some(line)
                }
                _ => None,
            }
        });

        if let Some(line) = res {
            return line;
        }
    }
}

use alloc::string::String;
use spin::{Mutex, MutexGuard};

const BACKSPACE: char = '\x08';
pub static mut STDIN: Stdin = Stdin::new();

/// function to handle the keys that entered
///
/// # Arguments
/// - `ch` - the char to handle
pub fn key_handle(ch: char) {
    let mut stdin = unsafe { STDIN.lock() };

    if ch == BACKSPACE {
        stdin.pop();
        // have to implement function that deletes the char
    } else {
        stdin.push(ch);
    }
}

pub struct Stdin {
    inner: Mutex<String>,
}

impl Stdin {
    /// creates new Stdin
    pub const fn new() -> Self {
        Self {
            inner: Mutex::new(String::new()),
        }
    }

    /// locks the inner
    pub fn lock(&self) -> MutexGuard<String> {
        self.inner.lock()
    }

    /// function that reads line and returns it
    ///
    /// # Returns
    /// the line it read
    pub fn read_line(&self, buf: &mut String) -> usize {
        loop {
            let res = x86_64::instructions::interrupts::without_interrupts(|| {
                let mut buffer = self.lock();
                match buffer.chars().next_back() {
                    Some('\n') => {
                        let line = buffer.clone();
                        buffer.clear();
                        Some(line)
                    }
                    _ => None,
                }
            });

            if let Some(line) = res {
                *buf = line.clone();
                return buf.len();
            }
        }
    }
}

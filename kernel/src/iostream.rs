use crate::mutex::{Mutex, MutexGuard};
use alloc::string::String;

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

    /// Read bytes from the standard input.
    ///
    /// # Arguments
    /// - `buf` - The buffer to read into.
    /// A maximum of `buf.len()` bytes will be read.
    ///
    /// # Returns
    /// The amount of bytes read.
    pub fn read(&self, buf: &mut [u8]) -> usize {
        let mut source = self.lock();
        let source_bytes = source.as_bytes();

        for i in 0..buf.len() {
            // Check if all bytes were read already.
            if i < source_bytes.len() {
                buf[i] = source_bytes[i];
            } else {
                *source = String::new();

                return i;
            }
        }
        *source = String::from(&source.as_str()[buf.len()..]);

        buf.len()
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

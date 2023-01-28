mod keycode;

use bitflags::bitflags;
use lazy_static::lazy_static;
use spin::Mutex;

/// PS/2 keyboard scancode wrapper
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Scancode(u8);

impl Scancode {
    /// function that returns the scancode as ASCII according to the arrays
    ///
    /// # Returns
    /// the character as u8, None if the value was not found
    fn to_ascii(&self) -> Option<u8> {
        match self.0 {
            0x01..=0x0e => Some(TO_ASCII_LOW[self.0 as usize - 0x01]),
            0x0f..=0x1c => Some(TO_ASCII_MID1[self.0 as usize - 0x0f]),
            0x1e..=0x28 => Some(TO_ASCII_MID2[self.0 as usize - 0x1e]),
            0x2c..=0x35 => Some(TO_ASCII_HIGH[self.0 as usize - 0x2c]),
            0x39 => Some(b' '),
            _ => None,
        }
    }
}

pub struct Keyboard {
    data_port: u16,
    pub state: Modifiers,
}

impl Keyboard {
    /// function that gets the scancode from 0x60 port
    /// inline because is single line and O(1) complexity
    ///
    /// # Returns
    /// the scancode
    #[inline]
    pub fn read_scancode(&self) -> Scancode {
        Scancode(unsafe { crate::io::inb(self.data_port) })
    }
}

const TO_ASCII_LOW: &'static [u8; 17] = b"\x1B1234567890-=\0x02";

const TO_ASCII_MID1: &'static [u8; 14] = b"\tqwertyuiop[]\n";

const TO_ASCII_MID2: &'static [u8; 11] = b"asdfghjkl;'";

const TO_ASCII_HIGH: &'static [u8; 10] = b"zxcvbnm,./";

bitflags! {
    pub struct Modifiers: u8 {
        const L_SHIFT  = 0b1000_0000;
        const R_SHIFT  = 0b0100_0000;
        const R_CTRL   = 0b0010_0000;
        const L_CTRL   = 0b0001_0000;
        const R_ALT    = 0b0000_1000;
        const L_ALT    = 0b0000_0100;
        const CAPSLOCK = 0b0000_0010;
        const NUMLOCK  = 0b0000_0001;
    }
}

impl Modifiers {
    /// function that checks is the shift is pressed
    /// inline because is single line and O(1) complexity
    ///
    /// # Returns
    /// if the shift is press returns true, false otherwise
    #[inline]
    pub fn is_shifted(&self) -> bool {
        self.contains(Modifiers::L_SHIFT) | self.contains(Modifiers::R_SHIFT)
    }

    /// function that checks is the char has to be uppercase
    /// inline because is single line and O(1) complexity
    ///
    /// # Returns
    /// if the char is uppercase returns true, false otherwise
    #[inline]
    pub fn is_uppercase(&self) -> bool {
        (self.contains(Modifiers::L_SHIFT) | self.contains(Modifiers::R_SHIFT))
            ^ self.contains(Modifiers::CAPSLOCK)
    }

    /// function that updates the modifiers state from a given scancode.
    ///
    /// # Arguments
    /// - `scancode` - the scancode
    fn update(&mut self, scancode: Scancode) {
        match scancode {
            Scancode(0x1D) => self.insert(Modifiers::L_CTRL),
            Scancode(0x2A) => self.insert(Modifiers::L_SHIFT),
            Scancode(0x36) => self.insert(Modifiers::R_SHIFT),
            Scancode(0x38) => self.insert(Modifiers::L_ALT),
            Scancode(0x3A) => self.toggle(Modifiers::CAPSLOCK),
            Scancode(0x9D) => self.remove(Modifiers::L_CTRL),
            Scancode(0xAA) => self.remove(Modifiers::L_SHIFT),
            Scancode(0xB6) => self.remove(Modifiers::R_SHIFT),
            Scancode(0xB8) => self.remove(Modifiers::L_ALT),
            _ => {}
        }
    }

    /// function to apply the keyboard's modifiers to an ASCII scancode.
    ///
    /// # Arguments
    /// - `ascii` - the code of the character
    ///
    /// # Returns
    /// the char
    fn modify(&self, ascii: u8) -> u8 {
        use keycode::{get_key_index, KEYMAP};

        if let Some(c) = KEYMAP.get(get_key_index(ascii) as usize) {
            if self.is_shifted() || (self.is_uppercase() && (c[0] as char).is_alphabetic()) {
                c[1] as u8
            } else {
                c[0] as u8
            }
        } else {
            b'\0'
        }
    }
}

lazy_static! {
    static ref KEYBOARD: Mutex<Keyboard> = Mutex::new(Keyboard {
        data_port: 0x60,
        state: Modifiers::empty(),
    });
}
pub fn read_char() -> Option<char> {
    let mut lock = KEYBOARD.lock();

    let code = lock.read_scancode();
    lock.state.update(code);

    code.to_ascii()
        .map(|ascii| lock.state.modify(ascii) as char)
}

pub extern "x86-interrupt" fn handler(stack_frame: &super::ExceptionStackFrame) {
    if let Some(input) = read_char() {
        crate::print!("{}", input);
    }
    // send the PICs the end interrupt signal
    unsafe {
        let mut pics = super::PICS.lock();
        pics.notify_end_of_interrupt(0x21);
    }
}

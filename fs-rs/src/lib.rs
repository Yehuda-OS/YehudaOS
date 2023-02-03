#![no_std]
#![feature(strict_provenance)]
#![feature(allocator_api)]

extern crate alloc;
use alloc::string::String;

pub mod fs;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {}
}

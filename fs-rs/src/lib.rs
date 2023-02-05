#![no_std]
#![feature(strict_provenance)]
#![feature(allocator_api)]

extern crate alloc;

pub mod fs;

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}

// #![no_std]
#![feature(strict_provenance)]
#![feature(allocator_api)]

extern crate alloc;

pub mod fs;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}

gnome-terminal -- rust-gdb -ex "target remote :1234" -ex "file target/x86_64-unknown-none/debug/kernel"
cargo run -- "-s -S"
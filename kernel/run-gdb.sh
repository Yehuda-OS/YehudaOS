gnome-terminal -- rust-gdb -ex "target remote :1234" \
-ex "file target/x86_64-unknown-none/debug/kernel" \
-ex "b _start" -ex "c" -ex "layout src"
cargo run -- "-s -S"
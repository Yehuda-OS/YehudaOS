#! /bin/bash
# builds and runs the kernel in qemu

set -xe

LIMINE_GIT_URL="https://github.com/limine-bootloader/limine.git"
KERNEL="target/kernel/debug/kernel"

# 1. build the kernel
cargo +nightly build

# 2. fetch and build limine
if [ ! -d target/limine ]; then
    git clone $LIMINE_GIT_URL --depth=1 --branch v3.0-branch-binary target/limine
fi
# Make sure we have an up-to-date version of the bootloader.
cd target/limine
git fetch
make
cd -

# 3. build the iso file
rm -rf iso_root
mkdir -p iso_root
cp $KERNEL \
conf/limine.cfg target/limine/limine.sys target/limine/limine-cd.bin \
target/limine/limine-cd-efi.bin iso_root/
xorriso -as mkisofs -b limine-cd.bin \
    -no-emul-boot -boot-load-size 4 -boot-info-table \
    --efi-boot limine-cd-efi.bin \
    -efi-boot-part --efi-boot-image --protective-msdos-label \
    iso_root -o $KERNEL.iso
target/limine/limine-deploy $KERNEL.iso
rm -rf iso_root

# 4. run the kernel
qemu-system-x86_64 -cdrom $KERNEL.iso

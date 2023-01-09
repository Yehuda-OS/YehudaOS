use core::arch::asm;

#[inline]
unsafe fn inb(port: u16) -> u8 {
    let res: u8;

    asm!(
    "in al, dx",
    out("al") (res),
    in("dx") (port),
    );

    res
}

#[inline]
unsafe fn inw(port: u16) -> u16 {
    let res: u16;

    asm!(
    "in ax, dx",
    out("ax") (res),
    in("dx") (port),
    );

    res
}

#[inline]
unsafe fn inl(port: u16) -> u32 {
    let res: u32;

    asm!(
    "in eax, dx",
    out("eax") (res),
    in("dx") (port),
    );

    res
}

#[inline]
pub unsafe fn outb(port: u16, value: u8) {
    asm!(
       "out dx, al",
       in("dx") port,
       in("al") value,
    );
}

#[inline]
unsafe fn outw(port: u16, value: u16) {
    asm!(
       "out dx, ax",
       in("dx") port,
       in("ax") value,
    );
}

#[inline]
unsafe fn outl(port: u16, value: u32) {
    asm!(
       "out dx, eax",
       in("dx") port,
       in("eax") value,
    );
}

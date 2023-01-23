use core::arch::asm;

#[inline]
pub unsafe fn inb(port: u16) -> u8 {
    let res: u8;

    asm!(
    "in al, dx",
    out("al") (res),
    in("dx") (port),
    );

    res
}

#[inline]
pub unsafe fn inw(port: u16) -> u16 {
    let res: u16;

    asm!(
    "in ax, dx",
    out("ax") (res),
    in("dx") (port),
    );

    res
}

#[inline]
pub unsafe fn inl(port: u16) -> u32 {
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
pub unsafe fn outw(port: u16, value: u16) {
    asm!(
       "out dx, ax",
       in("dx") port,
       in("ax") value,
    );
}

#[inline]
pub unsafe fn outl(port: u16, value: u32) {
    asm!(
       "out dx, eax",
       in("dx") port,
       in("eax") value,
    );
}

/// Write to a Model Specific Register.
///
/// # Arguments
/// - `msr` - The model specific register to write to.
/// - `data` - The data to write.
#[inline]
pub fn wrmsr(msr: u32, data: u64) {
    let low = data & (!0 as u32) as u64;
    let high = data >> 32;

    unsafe {
        asm!("
        wrmsr
        ", in("ecx")msr, in("edx")high, in("eax")low);
    }
}

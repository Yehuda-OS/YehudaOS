#[macro_export]
macro_rules! interrupt_handler {
    ($name:ident, $handler:ident) => {{
        #[no_mangle]
        pub extern "C" fn $name() -> ! {
            unsafe {
                asm!(
                    "
                    push rax
                    push rcx
                    push rdx
                    push rsi
                    push rdi
                    push r8
                    push r9
                    push r10
                    push r11
                    mov rdi, rsp
                    add rdi, 9*8
                    call {handler}
                    pop r11
                    pop r10
                    pop r9
                    pop r8
                    pop rdi
                    pop rsi
                    pop rdx
                    pop rcx
                    pop rax
                    iretq",
                    handler = in(reg) $handler,
                    options(noreturn),
                );
            }
        }

        $name
    }}
}

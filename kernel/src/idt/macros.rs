/// Save the general purpose registers of the process and run the handler.
#[macro_export]
macro_rules! interrupt_handler {
    ($handler:ident => $name:ident) => {{
        #[naked]
        #[no_mangle]
        pub extern "C" fn $name() -> ! {
            unsafe {
                asm!(
                    "
                    mov gs:0x0, rax
                    mov gs:0x8, rbx
                    mov gs:0x10, rcx
                    mov gs:0x18, rdx
                    mov gs:0x20, rsi
                    mov gs:0x28, rdi
                    mov gs:0x30, rbp
                    mov gs:0x38, r8
                    mov gs:0x40, r9
                    mov gs:0x48, r10
                    mov gs:0x50, r11
                    mov gs:0x58, r12
                    mov gs:0x60, r13
                    mov gs:0x68, r14
                    mov gs:0x70, r15

                    // Move the interrupt stack frame struct to `rdi` to send it as a parameter.
                    mov rdi, rsp
                    call {}
                    ",
                    sym $handler,
                    options(noreturn),
                );
            }
        }

        $name
    }}
}

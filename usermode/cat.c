#include "yehuda-os/sys.h"
#include "yehuda-os/helpers.h"

int main(int argc, char **argv)
{
    if (argc < 2)
    {
        print_str("cat: missing file operand");
        print_newline();
        print_str("Usage: cat <file>");
        print_newline();
        return 1;
    }
    int fd = open(argv[1]);
    if (fd == -1)
    {
        print_str("File does not exist.");
        print_newline();
        return 1;
    }
    struct Stat stat = {.directory = 0, .size = 0};
    fstat(fd, &stat);
    if (stat.directory == TRUE)
    {
        print_str("Specified path is not a file.");
        print_newline();
        return 1;
    }

    char buf[stat.size];
    read(fd, (void *)buf, stat.size, 0);
    print_str(buf);
    print_newline();

    free_array(&buf, stat.size);

    return 0;
}

// Tell the compiler incoming stack alignment is not RSP%16==8 or ESP%16==12
__attribute__((force_align_arg_pointer)) void _start()
{
    asm("call main");

    /* exit system call */
    asm("mov $0, %rdi;"
        "mov %eax, %edi;"
        "mov $0x3c, %rax;"
        "syscall");
    // tell the compiler to make sure side effects are done before the asm statement
    __builtin_unreachable();
}

#include "yehuda-os/helpers.h"
#include "yehuda-os/sys.h"

int main(int argc, char** argv)
{
    int fd           = 0;
    struct Stat stat = { .directory = 0, .size = 0 };
    char* buf        = NULL;

    if (argc <= 1)
    {
        print_str("cat: missing file operand\n"
                  "Usage: cat <file>\n");

        return 1;
    }

    fd = open(argv[1]);
    if (fd == -1)
    {
        print_str("cat: file does not exist\n");

        return 1;
    }
    fstat(fd, &stat);
    if (stat.directory == TRUE)
    {
        print_str("cat: specified path is not a file\n");

        return 1;
    }

    buf = malloc(stat.size + 1);
    read(fd, (void*)buf, stat.size, 0);
    buf[stat.size] = '\0';
    print_str(buf);
    print_newline();

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

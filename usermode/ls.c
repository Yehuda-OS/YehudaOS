#include "yehuda-os/helpers.h"
#include "yehuda-os/sys.h"

#define BUF_SIZE 1024

int main(int argc, char* argv[])
{
    int fd                  = open(argc > 1 ? "." : argv[1]);
    struct Stat ls_dir_stat = { .size = 0, .directory = 0 };
    struct Stat child_stat  = { .size = 0, .directory = 0 };
    struct DirEntry entry   = { .id = 0, .name = 0 };

    if (fstat(fd, &ls_dir_stat) == -1)
    {
        print_str("ls: directory does not exist\n");

        return 1;
    }

    for (size_t i = 0; i < ls_dir_stat.size; i++)
    {
        if (readdir(fd, 0, &entry) == -1 || fstat((int)entry.id, &child_stat) == -1)
        {
            print_str("ls: failed to read directory\n");

            return 1;
        }
        print_str(entry.name);
        if (child_stat.directory)
        {
            print_str("/");
        }
        print_newline();
    }

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
    // tell the compiler to make sure side effects are done before the asm
    // statement
    __builtin_unreachable();
}
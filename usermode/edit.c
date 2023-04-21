#include "yehuda-os/helpers.h"
#include "yehuda-os/sys.h"

int main(int argc, char *argv[])
{
    if (argc < 2)
    {
        print_str("edit: missing file operand");
        print_newline();
        print_str("Usage: edit <file>");
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

    char *curr_line = NULL;
    char content[1024] = "";

    while (1)
    {
        curr_line = getline();
        strcat(content, curr_line);
        strcat(content, " \n");

        if (strlen(curr_line) == 0 || curr_line == NULL)
        {
            break;
        }

        curr_line[0] = '\0';
    }

    write(fd, content, strlen(content), 0);
    free(curr_line);

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
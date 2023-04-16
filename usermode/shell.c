#include "yehuda-os/helpers.h"
#include "yehuda-os/sys.h"

const char* EXECUTABLE_PATH_START[] = { "./", "../", "/", NULL };
const char* BUILTINS[]              = { "cd" };

/**
 * Returns the amount of words in `str`.
 */
size_t count_words(const char* str)
{
    size_t count   = 0;
    bool_t in_word = FALSE;

    while (*str)
    {
        if (*str == ' ')
        {
            in_word = FALSE;
        }
        else if (!in_word)
        {
            in_word = TRUE;
            count++;
        }
        str++;
    }

    return count;
}

/**
 * Splits `command` into words separated by spaces.
 *
 * returns: An array of the words that are in the command,
 * terminated by a NULL pointer or `NULL` on failure.
 */
char** parse_command(const char* command)
{
    const char* start   = NULL;
    size_t word_len     = 0;
    const char* current = command;
    char** words        = calloc(count_words(command) + 1, sizeof(char*));
    bool_t in_word      = FALSE;
    size_t count        = 0;

    if (!words)
    {
        return NULL;
    }

    while (*current)
    {
        if (*current == ' ')
        {
            if (in_word)
            {
                words[count] = malloc((word_len + 1) * sizeof(char));
                if (!words[count])
                {
                    free_array((void**)words, count);
                    free(words);

                    return NULL;
                }
                strncpy(words[count], start, word_len);
                words[count][word_len] = '\0';

                count++;
                word_len = 0;
                in_word  = FALSE;
            }
        }
        else
        {
            if (!in_word)
            {
                in_word = TRUE;
                start   = current;
            }
            word_len++;
        }
        current++;
    }

    if (word_len > 0)
    {
        words[count] = malloc((word_len + 1) * sizeof(char));
        if (!words[count])
        {
            free_array((void**)words, count);
            free(words);

            return NULL;
        }
        strncpy(words[count], start, word_len);
        words[count][word_len] = '\0';
    }

    return words;
}

/**
 * Returns `TRUE` if a command is an executable file path or `FALSE` if it is a builtin.
 *
 * `command`: The command.
 */
bool_t is_executable(const char* command)
{
    const char** current_str     = EXECUTABLE_PATH_START;
    const char* path_start_index = NULL;
    const char* command_index    = NULL;

    while (*current_str)
    {
        path_start_index = *current_str;
        command_index    = command;
        while (*command_index)
        {
            if (!*path_start_index)
            {
                return TRUE;
            }
            if (*path_start_index != *command_index)
            {
                break;
            }
            command_index++;
            path_start_index++;
        }
        current_str++;
    }

    return FALSE;
}

/**
 * Handles a builtin command.
 */
void handle_builtin()
{
}

/**
 * Handles a command that executes a file.
 */
void handle_executable()
{
}

int main()
{
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

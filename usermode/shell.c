#include "yehuda-os/helpers.h"
#include "yehuda-os/sys.h"

const char* EXECUTABLE_PATH_START[] = { "./", "../", "/" };

/**
 * Reads a line from the console.
 *
 * returns: The line that was read or `NULL` on failure.
 */
char* get_command()
{
    ssize_t bytes_read = 0;
    size_t current     = 0;
    size_t len         = 1;
    char* buffer       = NULL;

    do
    {
        if (current == len - 1)
        {
            len *= 2;
            buffer = realloc(buffer, len);

            if (!buffer)
            {
                return NULL;
            }
        }

        bytes_read = read(stdin, buffer + current, 1, 0);
        if (bytes_read == -1)
        {
            free(buffer);

            return NULL;
        }
        else
        {
            current += bytes_read;
        }
    } while (buffer[current - bytes_read] != '\n');
    buffer[current - bytes_read] = '\0';

    return buffer;
}

/**
 * Returns the amoutn of words in `str`.
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
    // tell the compiler to make sure side effects are done before the asm statement
    __builtin_unreachable();
}

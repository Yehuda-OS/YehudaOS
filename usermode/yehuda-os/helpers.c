#include "helpers.h"
#include "sys.h"

size_t strlen(const char* s)
{
    size_t count = 0;

    while (*s)
    {
        count++;
        s++;
    }

    return count;
}

char* strcpy(char* destination, const char* source)
{
    char* ptr = destination;

    while (*source != '\0')
    {
        *destination = *source;
        destination++;
        source++;
    }
    *destination = '\0';

    return ptr;
}

char* strncpy(char* dest, const char* src, size_t n)
{
    size_t i;

    for (i = 0; i < n && src[i] != '\0'; i++)
    {
        dest[i] = src[i];
    }
    for (; i < n; i++)
    {
        dest[i] = '\0';
    }

    return dest;
}

int strcmp(const char* str1, const char* str2)
{
    int i = 0;

    while (str1[i] == str2[i])
    {
        if (str1[i] == '\0')
        {
            return 0;
        }
        i++;
    }

    return str1[i] > str2[i] ? 1 : -1;
}

int isspace(int c)
{
    return (c == ' ' || c == '\t' || c == '\n' || c == '\r' || c == '\f' || c == '\v');
}

/**
 * Free all the elements of an array of pointers `arr` with length of `size`.
 */
void free_array(void** arr, size_t size)
{
    for (size_t i = 0; i < size; i++)
    {
        free(arr[i]);
        arr[i] = NULL;
    }
}

/**
 * Reads a line from the console.
 *
 * returns: The line that was read or `NULL` on an allocation failure.
 *          The returned buffer must be freed by the caller.
 */
char* getline()
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

            if (buffer == NULL)
            {
                return NULL;
            }
        }

        bytes_read = read(STDIN, buffer + current, 1, 0);
        if (bytes_read == -1)
        {
            free(buffer);

            return NULL;
        }
        else if (bytes_read == 1)
        {
            if (buffer[current] == '\b')
            {
                if (current > 0)
                {
                    print_str("\b \b");
                    current--;
                }
            }
            else
            {
                write(STDOUT, buffer + current, 1, 0);
                current++;
            }
        }
    } while (buffer[current - bytes_read] != '\n');
    buffer[current - bytes_read] = '\0';

    return buffer;
}

/**
 * Print a string `str` to the screen.
 */
void print_str(const char* str)
{
    write(STDOUT, str, strlen(str), 0);
}

/**
 * Print the '\n' character.
 */
void print_newline()
{
    write(STDOUT, "\n", 1, 0);
}

/**
 * Convert an integer to a string.
 *
 * `num`: The number to convert.
 * `buffer`: The string to put the result into.
 *           Must be at least 11 bytes long.
 */
void int_to_string(int num, char* buffer)
{
    int i        = 0;
    int num_copy = 0;

    if (num == 0)
    {
        buffer[0] = '0';
        buffer[1] = '\0';

        return;
    }

    if (num < 0)
    {
        buffer[0] = '-';
        num       = -num;
        i         = 1;
    }

    num_copy = num;
    while (num_copy > 0)
    {
        num_copy /= 10;
        i++;
    }

    buffer[i] = '\0';
    while (num > 0)
    {
        i--;
        buffer[i] = '0' + (num % 10);
        num /= 10;
    }
}

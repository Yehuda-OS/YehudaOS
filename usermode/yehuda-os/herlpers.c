#include "helpers.h"

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

    if (destination == NULL)
    {
        return NULL;
    }

    while (*source)
    {
        *destination = *source;
        destination++;
        source++;
    }
    *destination = '\0';

    return ptr;
}

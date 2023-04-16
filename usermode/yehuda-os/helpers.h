#ifndef YEHUDAOS_HELPERS
#define YEHUDAOS_HELPERS
#include "sys.h"

size_t strlen(const char* s);

char* strcpy(char* destination, const char* source);

char* strncpy(char* dest, const char* src, size_t n);

void free_array(void** arr, size_t size);

char* getline();

void print_str(const char* str);

void int_to_string(int num, char* str);

#endif // YEHUDAOS_HELPERS

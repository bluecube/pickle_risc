#include "util.h"
#include "printing.h"

#include <stdio.h>

void* malloc_with_msg(size_t size, const char* label) {
    void *ret = malloc(size);
    if (!ret)
        error("Allocating %zuB for %s failed", size, label);

    return ret;
}

void* realloc_with_msg(void* ptr, size_t size, const char* label) {
    void *ret = realloc(ptr, size);
    if (!ret)
        error("Allocating %zuB for %s failed", size, label);

    return ret;
}

int parse_digit(int c) {
    if (c >= '0' && c <= '9')
        return c - '0';
    else if (c >= 'a' && c <= 'f')
        return c - 'a' + 10;
    else if (c >= 'A' && c <= 'F')
        return c - 'A' + 10;
    else
        return -1;
}

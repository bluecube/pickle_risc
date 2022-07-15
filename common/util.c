#include "util.h"
#include "printing.h"

#include <stdio.h>
#include <string.h>

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

char* strdup_with_msg(const char* s, const char* label) {
    size_t length = strlen(s) + 1;
    char *ret = malloc_with_msg(length, label);
    if (ret)
        memcpy(ret, s, length);
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

bool intervals_overlap(size_t start1, size_t end1, size_t start2, size_t end2) {
    return start1 < end2 && start2 < end1;
}

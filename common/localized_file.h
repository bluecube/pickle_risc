#pragma once
#include <stdbool.h>
#include <stdio.h>

struct location {
    const char* filename;
    unsigned line;
    unsigned column;
};

struct localized_file {
    FILE* fp;
    struct location location;

    int ungetcChar;
};

bool localized_file_open(struct localized_file *this, const char *filename);
void localized_file_close(struct localized_file *this);

bool localized_file_getc(struct localized_file *this, int *c);
void localized_file_ungetc(struct localized_file *this, int c);

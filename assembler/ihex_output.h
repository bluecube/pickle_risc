#pragma once

#include <stdint.h>
#include <stdio.h>
#include <stdbool.h>

struct ihex_output {
    const char* filename; // For error reporting
    uint16_t address; // Address of the first byte in buffer
    char buffer[16];
    uint16_t bufferUsed;
    FILE* fp;
};

bool ihex_output_open(struct ihex_output *state, const char *filename);
bool ihex_output_close(struct ihex_output *state);
bool ihex_output_byte(struct ihex_output *state, uint16_t address, char b);

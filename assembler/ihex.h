#pragma once

#include <stdint.h>
#include <stdio.h>
#include <stdbool.h>

struct ihex_writer {
    const char* filename; // For error reporting
    uint16_t address; // Address of the first byte in buffer
    uint8_t buffer[16];
    uint16_t bufferUsed;
    FILE* fp;
};

bool ihex_writer_open(const char *filename, struct ihex_writer *state);
bool ihex_writer_close(struct ihex_writer *state);
bool ihex_writer_write(uint16_t address, uint8_t byte, struct ihex_writer *state);

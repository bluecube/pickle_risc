#pragma once

#include "localized_file.h"
#include "stack.h"

#include <stdint.h>
#include <stdio.h>
#include <stdbool.h>

#define IHEX_RECORD_TYPE_DATA 0x00
#define IHEX_RECORD_TYPE_EOF 0x01

bool ihex_write_record(FILE *fp, uint16_t address, uint8_t recordType, char *data, uint16_t dataSize);
bool ihex_read_record(struct localized_file *f, uint16_t *address, uint8_t *recordType, char_stack_t *data, struct location *recordLocation);

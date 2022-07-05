#pragma once

#include <stdint.h>
#include <stdio.h>
#include <stdbool.h>

#define IHEX_RECORD_TYPE_DATA 0x00
#define IHEX_RECORD_TYPE_EOF 0x01

bool ihex_write_record(FILE *fp, uint16_t address, uint8_t recordType, char *data, uint16_t dataSize);

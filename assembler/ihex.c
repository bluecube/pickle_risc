#include "ihex.h"
#include "printing.h"

#include <assert.h>

static void error_writing() {
    error("writing ihex file failed");
}

static bool write_byte(FILE *fp, int byte, int *checksum) {
    byte = byte & 0xff;
    assert(byte >= 0);
    assert(byte <= 0xff);
    *checksum += byte;
    if (fprintf(fp, "%02x", byte) < 0) {
        error_writing();
        return false;
    } else
        return true;
}

bool ihex_write_record(FILE *fp, uint16_t address, uint8_t recordType, char *data, uint16_t dataSize) {
    if (fprintf(fp, ":") < 0) {
        error_writing();
        return false;
    }

    int checksum = 0;

    if (!write_byte(fp, dataSize, &checksum))
        return false;
    if (!write_byte(fp, address >> 8, &checksum))
        return false;
    if (!write_byte(fp, address, &checksum))
        return false;
    if (!write_byte(fp, recordType, &checksum))
        return false;
    for (unsigned i = 0; i < dataSize; ++i) {
        if (!write_byte(fp, data[i], &checksum))
            return false;
    }
    if (!write_byte(fp, -checksum, &checksum))
        return false;

    if (fprintf(fp, "\n") < 0) {
        error_writing();
        return false;
    }

    return true;
}

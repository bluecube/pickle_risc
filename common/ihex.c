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

static int read_byte(struct localized_file *f, uint8_t *checksum) {
    int ret = 0;
    for (int i = 0; i < 2; ++i) {
        ret *= 16;
        int c;
        if (!localized_file_getc(f, &c))
            return -1;

        if (c >= '0' && c <= '9')
            ret += c - '0';
        else if (c >= 'a' && c <= 'f')
            ret += c - 'a';
        else if (c >= 'A' && c <= 'F')
            ret += c - 'A';
        else {
            localized_error(f->location, "Unexpected character");
            return -1;
        }
    }
    *checksum += ret;
    return ret;
}

bool ihex_read_record(struct localized_file *f, uint16_t *address, uint8_t *recordType, char_stack_t *data, struct location *recordLocation) {
    while (true) {
        int c;
        if (!localized_file_getc(f, &c))
            return false;

        if (c == EOF) {
            localized_error(f->location, "Unexpected end of file");
            return false;
        } else if (c == ':')
            break;
    }

    *recordLocation = f->location;

    uint8_t checksum = 0;

    int b = read_byte(f, &checksum);
    if (b < 0)
        return false;

    data->used = b;
    if (!STACK_RESERVE(*data, (size_t)b))
        return false;

    b = read_byte(f, &checksum);
    if (b < 0)
        return false;
    *address = b << 8;
    b = read_byte(f, &checksum);
    if (b < 0)
        return false;
    *address |= b;
    b = read_byte(f, &checksum);
    if (b < 0)
        return false;
    *recordType |= b;

    for (size_t i = 0; i < data->used; ++i) {
        b = read_byte(f, &checksum);
        if (b < 0)
            return false;
        STACK_AT(*data, i) = b;
    }

    b = read_byte(f, &checksum);
    if (b < 0)
        return false;

    if (checksum != 0) {
        localized_error(*recordLocation, "Invalid record checskum");
        return false;
    }

    return true;
}

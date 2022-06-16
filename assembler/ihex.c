#include "ihex.h"
#include "printing.h"

#include <stdarg.h>
#include <assert.h>

#define RECORD_TYPE_DATA 0x00
#define RECORD_TYPE_EOF 0x01

__attribute__ ((format (printf, 2, 3)))
static bool output(struct ihex_writer *state, const char *format, ...) {
    va_list ap;
    va_start(ap, format);
    int ret = vfprintf(state->fp, format, ap);
    va_end(ap);

    if (ret < 0)
        error("%s: writing file failed", state->filename);

    return ret >= 0;
}

static bool output_byte(struct ihex_writer *state, int byte, int *checksum) {
    byte = byte & 0xff;
    assert(byte >= 0);
    assert(byte <= 0xff);
    *checksum += byte;
    return output(state, "%02x", byte);
}

static bool output_record(struct ihex_writer *state, uint16_t address, uint8_t recordType, uint8_t *data, uint16_t dataSize) {
    if (!output(state, ":"))
        return false;

    int checksum = 0;

    if (!output_byte(state, dataSize, &checksum))
        return false;
    if (!output_byte(state, address >> 8, &checksum))
        return false;
    if (!output_byte(state, address, &checksum))
        return false;
    if (!output_byte(state, recordType, &checksum))
        return false;
    for (unsigned i = 0; i < dataSize; ++i) {
        if (!output_byte(state, data[i], &checksum))
            return false;
    }
    if (!output_byte(state, -checksum, &checksum))
        return false;
    if (!output(state, "\n"))
        return false;

    return true;
}

static bool flush(struct ihex_writer *state) {
    if (!state->bufferUsed)
        return true;

    bool ret = output_record(state, state->address, RECORD_TYPE_DATA, state->buffer, state->bufferUsed);

    state->address += state->bufferUsed;
    state->bufferUsed = 0;

    return ret;
}

bool ihex_writer_open(const char *filename, struct ihex_writer *state) {
    state->address = 0;
    state->bufferUsed = 0;
    state->fp = fopen(filename, "w");
    if (!state->fp) {
        error("%s: Failed to open file", filename);
        return false;
    }

    return true;
}

bool ihex_writer_close(struct ihex_writer *state) {
    if (!state->fp)
        return true;

    bool ret = flush(state);
    ret = ret && output_record(state, 0x0000, RECORD_TYPE_EOF, NULL, 0);
    fclose(state->fp);

    state->fp = NULL;
    return ret;
}

bool ihex_writer_write(uint16_t address, uint8_t byte, struct ihex_writer *state) {
    if (address != state->address + state->bufferUsed) {
        if (!flush(state))
            return false;
        state->address = address;
    }
    state->buffer[(state->bufferUsed)++] = byte;
    if (state->bufferUsed == sizeof(state->buffer))
        return flush(state);
    return true;
}

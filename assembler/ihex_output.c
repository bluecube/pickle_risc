#include "ihex_output.h"
#include "ihex.h"
#include "printing.h"

static bool flush(struct ihex_output *state) {
    if (!state->bufferUsed)
        return true;

    bool ret = ihex_write_record(
        state->fp,
        state->address,
        IHEX_RECORD_TYPE_DATA,
        state->buffer,
        state->bufferUsed
    );

    state->address += state->bufferUsed;
    state->bufferUsed = 0;

    return ret;
}

bool ihex_output_open(struct ihex_output *state, const char *filename) {
    state->address = 0;
    state->bufferUsed = 0;
    state->fp = fopen(filename, "w");
    if (!state->fp) {
        error("%s: Failed to open file", filename);
        return false;
    }

    return true;
}

bool ihex_output_close(struct ihex_output *state) {
    if (!state->fp)
        return true;

    bool ret = flush(state);
    ret = ret && ihex_write_record(state->fp, 0x0000, IHEX_RECORD_TYPE_EOF, NULL, 0);
    fclose(state->fp);

    state->fp = NULL;
    return ret;
}

bool ihex_output_byte(struct ihex_output *state, uint16_t address, char b) {
    if (address != state->address + state->bufferUsed) {
        if (!flush(state))
            return false;
        state->address = address;
    }
    state->buffer[(state->bufferUsed)++] = b;
    if (state->bufferUsed == sizeof(state->buffer))
        return flush(state);
    return true;
}

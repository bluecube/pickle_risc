#include "memory.h"

#include "../../common/util.h"
#include "../../common/printing.h"
#include "../../common/ihex.h"

#include <assert.h>
#include <stdbool.h>
#include <stdlib.h>
#include <stdio.h>
#include <string.h>

#define CHUNK_MAX_GAP 32

struct range {
    physical_address_t start;
    physical_address_t end;
};

typedef STACK_DECLARATION(word_t) word_stack_t;

static uint16_t read_fun(void *mappedBuffer, physical_offset_t offset) {
    return ((word_t *)mappedBuffer)[offset];
}

static void write_fun(void *mappedBuffer, physical_offset_t offset, word_t value) {
    ((word_t *)mappedBuffer)[offset] = value;
}

static void no_write_fun(void *mappedBuffer, physical_offset_t offset, word_t value) {
    (void)mappedBuffer;
    (void)offset;
    (void)value;
    // TODO: Trap somehow
}

/// Copy a byte array into a word array, storing data in big endian.
/// Handles unaligned access
static void copy_bytes_to_words(word_t *dest, char *data, size_t dataOffsetBytes, size_t dataLengthBytes) {
    if (dataLengthBytes == 0)
        return;

    if (dataOffsetBytes % 2) {
        size_t i = dataOffsetBytes / 2;
        dest[i] = (dest[i] & 0xff00) | data[0];

        data += 1;
        dataOffsetBytes += 1;
        dataLengthBytes -= 1;

        if (dataLengthBytes == 0)
            return;
    }

    size_t firstWord = dataOffsetBytes / 2;
    size_t wordsToCopy = dataLengthBytes / 2;
    for (size_t i = 0; i < wordsToCopy; ++i)
        dest[i + firstWord] = (data[2 * i] << 8) | data[2 * i + 1];

    if (dataLengthBytes % 2) {
        size_t i = firstWord + wordsToCopy;
        dest[i] = data[dataLengthBytes - 1] << 8 | (dest[i] & 0xff);
    }
}

static word_t *load_ihex(const char* filename, physical_size_t *size) {
    word_t *ret = NULL;

    struct localized_file f;
    if (!localized_file_open(&f, filename))
        return NULL;

    char_stack_t lineBuffer;
    if (!STACK_INIT(lineBuffer, 32))
        goto cleanup1;

    STACK_DECLARATION(word_t) dataBuffer;
    if (!STACK_INIT(dataBuffer, 1024))
        goto cleanup2;

    while (true) {
        uint16_t recordStart;
        uint8_t recordType;
        struct location recordLocation;
        if (!ihex_read_record(&f, &recordStart, &recordType, &lineBuffer, &recordLocation))
            goto cleanup3;

        if (recordType == IHEX_RECORD_TYPE_EOF)
            break;
        else if (recordType != IHEX_RECORD_TYPE_DATA) {
            localized_error(recordLocation, "Unsupported record type");
            goto cleanup3;
        }

        if (!lineBuffer.used)
            continue; // Skip empty records

        uint16_t recordEnd = recordStart + lineBuffer.used;
        uint16_t recordEndWordAddress = (recordEnd + sizeof(word_t) - 1) / sizeof(word_t);

        STACK_RESIZE(dataBuffer, recordEndWordAddress);

        copy_bytes_to_words(dataBuffer.ptr, lineBuffer.ptr, recordStart, lineBuffer.used);
    }

    ret = dataBuffer.ptr;
    dataBuffer.ptr = NULL; // Instead of deinitializing the stack we just steal its value
    *size = dataBuffer.used;

cleanup3:
    STACK_DEINIT(dataBuffer);
cleanup2:
    STACK_DEINIT(lineBuffer);
cleanup1:
    localized_file_close(&f);
    return ret;
}

bool dev_memory_init_buffer(
    struct dev_memory *memory,
    physical_address_t mappingStart,
    physical_size_t size,
    word_t *buffer,
    bool writable,
    struct cpu_state *cpuState
) {
    memory->data = buffer;
    memory->cpuState = cpuState;

    struct memory_mapping mapping = {
        .start = mappingStart,
        .end = mappingStart + size,
        .read = read_fun,
        .write = writable ? write_fun : no_write_fun,
        .mappingData = buffer,
        .mappingId = -1 // Will be set when adding the mapping
    };
    memory->mappingHandle = cpu_state_add_memory_mapping(cpuState, mapping);

    bool successful = memory->mappingHandle >= 0;
    if (!successful) {
        free(memory->data);
        memory->data = NULL;
    }
    return successful;
}

bool dev_memory_init_uninitialized(
    struct dev_memory *memory,
    physical_address_t mappingStart,
    physical_size_t size,
    bool writable,
    struct cpu_state *cpuState
) {
    word_t *buffer = malloc_with_msg(sizeof(word_t) * size, "memory device buffer");
    if (!buffer)
        return false;

    return dev_memory_init_buffer(memory, mappingStart, size, buffer, writable, cpuState);
}

bool dev_memory_init_ihex(
    struct dev_memory *memory,
    physical_address_t mappingStart,
    const char *imageFilename,
    bool writable,
    struct cpu_state *cpuState
) {
    physical_size_t size;
    word_t *data = load_ihex(imageFilename, &size);
    if (!data)
        return false;

    return dev_memory_init_buffer(memory, mappingStart, size, data, writable, cpuState);
}

void dev_memory_deinit(struct dev_memory *memory) {
    free(memory->data);
    memory->data = NULL;

    if (memory->mappingHandle >= 0)
        cpu_state_remove_memory_mapping(memory->cpuState, memory->mappingHandle);
}

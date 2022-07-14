#pragma once
#include "../cpu_state.h"
#include "../../common/stack.h"

struct dev_memory {
    word_t *data;
    int mappingHandle;
    struct cpu_state *cpuState;
};

/// Init the memory from an existing buffer. Takes ownership
bool dev_memory_init_buffer(
    struct dev_memory *memory,
    physical_address_t mappingStart,
    physical_size_t size,
    word_t *buffer,
    bool writable,
    struct cpu_state *cpuState
);

bool dev_memory_init_uninitialized(
    struct dev_memory *memory,
    physical_address_t mappingStart,
    physical_size_t size,
    bool writable,
    struct cpu_state *cpuState
);

bool dev_memory_init_ihex(
    struct dev_memory *memory,
    physical_address_t mappingStart,
    const char *imageFilename,
    bool writable,
    struct cpu_state *cpuState
);

void dev_memory_deinit(struct dev_memory *memory);

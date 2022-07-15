#pragma once
#include "../common/stack.h"

#include <stdint.h>
#include <stdbool.h>

#define EMULATOR_TRAP_BREAK 1 ///< Break instruction encountered

typedef uint32_t physical_address_t;
typedef uint32_t physical_size_t;
typedef int32_t physical_offset_t;
typedef uint16_t word_t;

#define PHYSICAL_ADDRESS_FORMAT "0x%06x"

struct memory_mapping {
    physical_address_t start;
    physical_address_t end;

    word_t (*read)(void *data, physical_offset_t offset);
    void (*write)(void *data, physical_offset_t offset, word_t value);
    void *mappingData;

    int mappingId;
};

struct cpu_state {
    uint16_t reg[8];
    uint16_t pc;

    uint16_t contextId;
    uint16_t intPc;
    uint16_t intCause;
    uint16_t mmuAddr;
    uint16_t tmp1;
    uint16_t tmp2;
    uint16_t aluFlags;

    uint16_t instruction;
    uint16_t latchedInstruction;

    STACK_DECLARATION(struct memory_mapping) physicalMemory;
    int nextMappingId;
};

bool cpu_state_init(struct cpu_state *state);
void cpu_state_deinit(struct cpu_state *state);

/// Add memory mapping, return its handle.
/// After mappings are inserted, they need to be sorted in cpu_state_reset().
int cpu_state_add_memory_mapping(struct cpu_state *state, struct memory_mapping mapping);
void cpu_state_remove_memory_mapping(struct cpu_state *state, int mappingHandle);

/// Prepare the CPU to run.
/// Resets the state and prepares the internal structures (=sorts physical memory mappings)
void cpu_state_reset(struct cpu_state *state);
/// Perform a single clock cycle of the CPU, returns 0 or a number of encountered emulator trap
int cpu_state_step(struct cpu_state *state);

#include "cpu_state.h"

#include "../common/util.h"
#include "../common/printing.h"

#include <stdlib.h>

static void reset_registers(struct cpu_state *state) {
    for (size_t i = 0; i < ARRAY_SIZE(state->reg); ++i)
        state->reg[i] = 0;

    state->pc = 0;

    state->contextId = 0;
    state->intPc = 0;
    state->intCause = 0;
    state->mmuAddr = 0;
    state->tmp1 = 0;
    state->tmp2 = 0;
    state->aluFlags = 0;

    state->instruction = 0;
    state->latchedInstruction = 0;
}

bool cpu_state_init(struct cpu_state *state) {
    reset_registers(state);

    if (!STACK_INIT(state->physicalMemory, 4))
        return false;
    state->nextMappingId = 0;

    return true;
}

void cpu_state_deinit(struct cpu_state *state) {
    STACK_DEINIT(state->physicalMemory);
}

int cpu_state_add_memory_mapping(struct cpu_state *state, struct memory_mapping mapping) {
    for (size_t i = 0; i < state->physicalMemory.used; ++i) {
        struct memory_mapping *otherMapping = &STACK_AT(state->physicalMemory, i);
        if (intervals_overlap(mapping.start, mapping.end, otherMapping->start, otherMapping->end)) {
            error(
                "Physical memory mapping " PHYSICAL_ADDRESS_FORMAT "-" PHYSICAL_ADDRESS_FORMAT
                " overlaps with previous mapping "
                PHYSICAL_ADDRESS_FORMAT "-" PHYSICAL_ADDRESS_FORMAT,
                mapping.start, mapping.end, otherMapping->start, otherMapping->end
            );
            return -1;
        }
    }

    if (!STACK_PUSH(state->physicalMemory, mapping))
        return -1;
    STACK_AT_R(state->physicalMemory, 0).mappingId = state->nextMappingId;
    return state->nextMappingId++;
}

void cpu_state_remove_memory_mapping(struct cpu_state *state, int mappingHandle) {
    for (size_t i = 0; i < state->physicalMemory.used; ++i) {
        if (STACK_AT(state->physicalMemory, i).mappingId == mappingHandle) {
            STACK_AT(state->physicalMemory, i) = STACK_AT_R(state->physicalMemory, 0);
            --state->physicalMemory.used;
            return;
        }
    }
}

static int memory_mapping_comparator(const void *a, const void *b) {
    physical_address_t startA = ((struct memory_mapping *)a)->start;
    physical_address_t startB = ((struct memory_mapping *)b)->start;

    if (startA < startB)
        return -1;
    if (startA > startB)
        return 1;
    else
        return 0;
}

void cpu_state_reset(struct cpu_state *state) {
    reset_registers(state);

    qsort(
        state->physicalMemory.ptr,
        state->physicalMemory.used,
        sizeof(struct memory_mapping),
        memory_mapping_comparator
    );
}

int cpu_state_step(struct cpu_state *state) {
    
}

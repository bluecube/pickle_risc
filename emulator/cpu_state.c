#include "cpu_state.h"

#include "../common/util.h"

bool cpu_state_init(struct cpu_state *state) {
    for (uint16_t i = 0; i < ARRAY_SIZE(state->reg); ++i)
        state->reg[i] = 0;

    state->pc = 0;

    state->contextId = 0;
    state->intPc = 0;
    state->intCause = 0;
    state->mmuAddr = 0;
    state->tmp1 = 0;
    state->tmp2 = 0;
    state->aluFlags = 0;


    if (!STACK_INIT(state->physicalMemory, 4))
        return false;

    return true;
}

void cpu_state_deinit(struct cpu_state *state) {
    STACK_DEINIT(state->physicalMemory);
}

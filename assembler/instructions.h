#pragma once
#include <stdint.h>

#define INSTRUCTION_ARG_NONE 0
#define INSTRUCTION_ARG_GPR 1
#define INSTRUCTION_ARG_CR 2
#define INSTRUCTION_ARG_SIGNED 4
#define INSTRUCTION_ARG_UNSIGNED 5

struct instruction_argument {
    uint_fast8_t type;
    uint_fast8_t shift;
    uint_fast8_t size;
};

struct instruction {
    const char* mnemonic;
    uint16_t encoding;
    struct instruction_argument args[4];
};

extern struct instruction instructions[];

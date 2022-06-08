#pragma once

#include <stdint.h>
#include <stdbool.h>
#include "tokenizer.h"

struct symbol;

struct assembler_state {
    uint16_t pc;
    struct symbol* symtable; // TODO: Use self-organizing list, just for fun (but measure before and after!)
};

void assembler_state_init(struct assembler_state* state);
void assembler_state_deinit(struct assembler_state* state);
void assembler_state_before_pass(int pass, struct assembler_state* state);
bool assemble_multiple_files(int pass, int fileCount, char** filePaths, struct assembler_state* state);

#pragma once

#include "tokenizer.h"

#include <stdint.h>
#include <stdbool.h>

struct symbol {
    char* name;
    uint16_t address;
    bool defined;
    struct symbol* next;
};

struct assembler_state {
    uint16_t pc;
    struct symbol* symtable; // TODO: Use self-organizing list, just for fun (but measure before and after!)
    int pass;
};

void assembler_state_init(struct assembler_state* state);
void assembler_state_deinit(struct assembler_state* state);
void assembler_state_start_pass(int pass, struct assembler_state* state);
bool assemble_multiple_files(int fileCount, char** filePaths, struct assembler_state* state);
bool get_symbol_value(struct token* idToken, struct assembler_state* state, uint16_t* ret);

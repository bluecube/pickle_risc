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
};

void assembler_state_init(struct assembler_state* state);
void assembler_state_deinit(struct assembler_state* state);
void assembler_state_before_pass(int pass, struct assembler_state* state);
bool assemble_multiple_files(int pass, int fileCount, char** filePaths, struct assembler_state* state);
struct symbol* lookup_or_create_symbol(char* name, struct assembler_state* state);

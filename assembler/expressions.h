#pragma once

#include "tokenizer.h"

#include <stdbool.h>

struct assembler_state;

struct expression_operator;

/// Parse an expression from the input and evaluate it.
/// Parameter startLocation returns the beginning of the expression (even on error), can be NULL.
/// @return true if ret was set, false on error
bool evaluate_expression(struct assembler_state* state, struct tokenizer_state* tokenizer, numeric_value_t* ret, struct location* startLocation);

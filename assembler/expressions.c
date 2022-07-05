#include "expressions.h"
#include "assembler.h"

#include "../common/printing.h"

#include <assert.h>

#define GEN_BINARY_OP(name, expr) \
static numeric_value_t name(numeric_value_t v1, numeric_value_t v2) { \
    return expr; \
}

#define GEN_UNARY_OP(name, expr) \
static numeric_value_t name(numeric_value_t, numeric_value_t v) { \
    return expr; \
}

struct operator {
    int tokenType;
    int priority;
    unsigned arity;
    numeric_value_t (*fun)(numeric_value_t, numeric_value_t);
};

struct op_stack_element {
    struct operator* op;
    struct location location;
};

typedef STACK_DECLARATION(numeric_value_t) numeric_value_stack_t;
typedef STACK_DECLARATION(struct op_stack_element) operator_stack_t;

GEN_BINARY_OP(logical_or, v1 || v2)
GEN_BINARY_OP(logical_and, v1 && v2)
GEN_BINARY_OP(bitwise_or, v1 | v2)
GEN_BINARY_OP(bitwise_xor, v1 ^ v2)
GEN_BINARY_OP(bitwise_and, v1 & v2)
GEN_BINARY_OP(eq, v1 == v2)
GEN_BINARY_OP(neq, v1 != v2)
GEN_BINARY_OP(lt, v1 < v2)
GEN_BINARY_OP(gt, v1 > v2)
GEN_BINARY_OP(le, v1 <= v2)
GEN_BINARY_OP(ge, v1 >= v2)
GEN_BINARY_OP(shl, v1 << v2)
GEN_BINARY_OP(shr, v1 >> v2)
GEN_BINARY_OP(add, v1 + v2)
GEN_BINARY_OP(sub, v1 - v2)
GEN_BINARY_OP(mul, v1 * v2)
GEN_BINARY_OP(divide, v1 / v2)
GEN_BINARY_OP(mod, v1 % v2)
GEN_UNARY_OP(logical_not, !v)
GEN_UNARY_OP(bitwise_not, ~v)
GEN_UNARY_OP(unary_plus, v)
GEN_UNARY_OP(unary_minus, -v)

static struct operator operators[] = {
    { TOKEN_OPERATOR_LOGICAL_OR, 0, 2, logical_or },
    { TOKEN_OPERATOR_LOGICAL_AND, 1, 2, logical_and },
    { '|', 2, 2, bitwise_or },
    { '^', 3, 2, bitwise_xor },
    { '&', 4, 2, bitwise_and },
    { TOKEN_OPERATOR_EQ, 5, 2, eq },
    { TOKEN_OPERATOR_NEQ, 5, 2, neq },
    { '<', 5, 2, lt },
    { '>', 5, 2, gt },
    { TOKEN_OPERATOR_LE, 5, 2, le },
    { TOKEN_OPERATOR_GE, 5, 2, ge },
    { TOKEN_OPERATOR_SHL, 6, 2, shl },
    { TOKEN_OPERATOR_SHR, 6, 2, shr },
    { '+', 7, 2, add },
    { '-', 7, 2, sub },
    { '*', 8, 2, mul },
    { '/', 8, 2, divide },
    { '%', 8, 2, mod },
    { '!', 9, 1, logical_not },
    { '~', 9, 1, bitwise_not },
    { '+', 9, 1, unary_plus },
    { '-', 9, 1, unary_minus },
    { '(', 10, 0, NULL }, // Should never be popped separately, only with ')'
    { .tokenType = TOKEN_NONE }
};

static struct operator* find_operator(int tokenType, bool findBinaryOp) {
    struct operator* op = operators;

    while (op->tokenType != TOKEN_NONE) {
        if (op->tokenType == tokenType && (findBinaryOp == (op->arity == 2)))
            return op;
        ++op;
    }

    return NULL;
}

static bool pop_operator(operator_stack_t* operatorStack, numeric_value_stack_t* valueStack) {
    assert(operatorStack->used >= 1);

    struct op_stack_element op = STACK_AT_R(*operatorStack, 0);
    --(operatorStack->used);

    if (valueStack->used < op.op->arity) {
        localized_error(op.location, "Invalid syntax: Not enough values on stack");
        return false;
    }

    numeric_value_t v2 = STACK_AT_R(*valueStack, 0);
    numeric_value_t v1 = 0;
    if (op.op->arity == 2) {
        --valueStack->used;
        v1 = STACK_AT_R(*valueStack, 0);
    } else
        assert(op.op->arity == 1);

    STACK_AT_R(*valueStack, 0) = op.op->fun(v1, v2);

    return true;
}

bool evaluate_expression(struct assembler_state* state, struct tokenizer_state* tokenizer, numeric_value_t* ret, struct location* startLocation) {
    if (startLocation)
        *startLocation = peek_token(tokenizer)->location;

    bool successful = false;

    numeric_value_stack_t valueStack;
    if (!STACK_INIT(valueStack, 16))
        goto cleanup;
    operator_stack_t operatorStack;
    if (!STACK_INIT(operatorStack, 16)) {
        goto cleanup;
    }

    bool precededByValue = false; // Flag for determining whether the operator is binary or unary
    while (true) {
        struct token *nextToken = peek_token(tokenizer);
        if (nextToken->type == TOKEN_ERROR) {
            goto cleanup;
        } else if (nextToken->type == TOKEN_NUMBER) {
            if (precededByValue) {
                localized_error(nextToken->location, "Invalid syntax: Number preceeded by value");
                goto cleanup;
            }
            struct token token = get_token(tokenizer);
            assert(token.content == NULL); // Does not need to be freed

            numeric_value_t value = token.contentNumeric;
            if (!STACK_PUSH(valueStack, value))
                goto cleanup;
            precededByValue = true;
        } else if (nextToken->type == TOKEN_IDENTIFIER) {
            if (precededByValue) {
                localized_error(nextToken->location, "Invalid syntax: Identifier precedd by value");
                goto cleanup;
            }
            struct token token = get_token(tokenizer);
            uint16_t value;
            if (!get_symbol_value(&token, state, &value)) // Frees the token
                goto cleanup;
            if (!STACK_PUSH(valueStack, value))
                goto cleanup;
            precededByValue = true;
        } else if (nextToken->type == ')') {
            struct token token = get_token(tokenizer);
            assert(token.content == NULL); // Does not need to be freed

            while (operatorStack.used > 0 && STACK_AT_R(operatorStack, 0).op->tokenType != '(') {
                if (!pop_operator(&operatorStack, &valueStack))
                    goto cleanup;
            }

            if (operatorStack.used == 0) {
                localized_error(token.location, "Invalid syntax: Unexpected ')'");
                goto cleanup;
            }

            assert(STACK_AT_R(operatorStack, 0).op->tokenType == ')');
            --operatorStack.used;

            precededByValue = true;
        } else {
            struct operator* op = find_operator(nextToken->type, precededByValue);
            if (!op)
                break; // Unknown token, stop expression parsing

            struct token token = get_token(tokenizer);
            assert(token.content == NULL); // Does not need to be freed

            while (operatorStack.used > 0) {
                struct operator* stackTopOp = STACK_AT_R(operatorStack, 0).op;
                if (
                    stackTopOp->tokenType == '(' ||
                    (
                        // Unary operators are right associative, binary operators left
                        stackTopOp->arity == 2 ?
                        stackTopOp->priority < op->priority :
                        stackTopOp->priority <= op->priority
                    )
                )
                    break;
                if (!pop_operator(&operatorStack, &valueStack))
                    goto cleanup;
            }

            struct op_stack_element op2 = {op, token.location};
            if (!STACK_PUSH(operatorStack, op2))
                goto cleanup;

            precededByValue = false;
        }
    }

    while (operatorStack.used > 0)
        if (!pop_operator(&operatorStack, &valueStack))
            goto cleanup;

    successful = (valueStack.used == 1);
    if (!successful)
        localized_error(peek_token(tokenizer)->location, "Invalid syntax: %zu values left on stack", valueStack.used);
    else
        *ret = STACK_AT(valueStack, 0);

cleanup:
    STACK_DEINIT(operatorStack);
    STACK_DEINIT(valueStack);
    return successful;
}

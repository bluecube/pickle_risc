#include "assembler.h"
#include "tokenizer.h"
#include "pseudo_instructions.h"
#include "printing.h"
#include "expressions.h"

#include <string.h>

static bool process_db(struct assembler_state *state, struct tokenizer_state *tokenizer) {
    uint16_t buffer = 0;
    bool bufferHasValue = false;


    while (true) {
        numeric_value_t v;
        if (!evaluate_expression(state, tokenizer, &v, NULL))
            return false;

        if (bufferHasValue) {
            buffer |= v & 0xff;
            if (!assembler_output_word(buffer, state))
                return false;
            bufferHasValue = false;
        } else {
            buffer = (v & 0xff) << 8;
            bufferHasValue = true;
        }

        struct token token = get_token(tokenizer);
        switch (token.type) {
        case TOKEN_ERROR:
            free_token(&token);
            return false;
        case TOKEN_EOL:
        case TOKEN_EOF:
            free_token(&token);
            if (bufferHasValue)
                if (!assembler_output_word(buffer, state))
                    return false;
            return true;
        case ',':
            free_token(&token);
            break;
        default:
            localized_error(token.location, "Unexpected input");
            free_token(&token);
        }
    }
}

bool process_pseudo_instruction(struct token* mnemonicToken, struct assembler_state* state, struct tokenizer_state* tokenizer) {
    bool ret;
    if (!strcmp(mnemonicToken->content, ".db"))
        ret = process_db(state, tokenizer);
    /*else if (!strcmp(mnemonicToken->content, ".dw"))
        ret = process_dw(state, tokenizer);*/
    else {
        localized_error(mnemonicToken->location, "Invalid pseudo-instruction `%s`", mnemonicToken->content);
        ret = false;
    }

    free_token(mnemonicToken);
    return ret;
}

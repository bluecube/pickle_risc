#include "assembler.h"
#include "printing.h"

int main(int argc, char** argv) {
    if (argc < 2) {
        error("Need at least one file as argument\n");
        return EXIT_FAILURE;
    }

    struct assembler_state state;
    assembler_state_init(&state);

    for (int pass = 1; pass <= 2; ++pass) {
        if (!assembler_state_start_pass(pass, &state)) {
            assembler_state_deinit(&state);
            return EXIT_FAILURE;
        }
        if (!assemble_multiple_files(argc - 1, argv + 1, &state)) {
            assembler_state_deinit(&state);
            return EXIT_FAILURE;
        }
    }

    assembler_state_deinit(&state);
    return EXIT_SUCCESS;
}

#include "assembler.h"
#include "ihex.h"
#include "printing.h"

#include <stdbool.h>
#include <getopt.h>

void usage(char* command) {
    printf("Usage: %s [option]... file...\n", command);
    puts("Pickle risc assembler");
    puts("");
    puts("Options:");
    puts("	-o		Write output to this file instead of the default a.out");
    puts("	-h, --help	Print this message");
}

int main(int argc, char** argv) {
    bool verboseFlag = false;
    const char *outputFile = "a.out";

    struct option longOptions[] = {
        {"verbose", no_argument, 0, 'v'},
        {"output", required_argument, 0, 'o'},
        {"help", no_argument, 0, 'h'},
        {0, 0, 0, 0}
    };

    while (true) {
        int c = getopt_long(argc, argv, "vo:", longOptions, NULL);

        if (c == -1)
            break;

        switch (c) {
        case 0:
            break;
        case 'v':
            verboseFlag = true;
            break;
        case 'o':
            outputFile = optarg;
            break;
        case 'h':
            usage(argv[0]);
            return EXIT_SUCCESS;

        default:
            return EXIT_FAILURE;
        }
    }

    int inputCount = argc - optind;
    char** inputs = argv + optind;

    if (inputCount == 0) {
        fprintf(stderr, "%s: no input files\n", argv[0]);
        return EXIT_FAILURE;
    }

    struct assembler_state state;
    if (!assembler_state_init(outputFile, &state))
        return EXIT_FAILURE;

    state.verbose = verboseFlag;

    for (int pass = 1; pass <= 2; ++pass) {
        if (!assembler_state_start_pass(pass, &state)) {
            assembler_state_deinit(&state); // Ignoring return value
            return EXIT_FAILURE;
        }
        if (!assemble_multiple_files(inputCount, inputs, &state)) {
            assembler_state_deinit(&state); // Ignoring return value
            return EXIT_FAILURE;
        }
    }

    if (!assembler_state_deinit(&state))
        return EXIT_FAILURE;
    return EXIT_SUCCESS;
}

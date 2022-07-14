#include "devices/memory.h"
#include "devices/uart.h"
#include "cpu_state.h"

#include <stdbool.h>
#include <getopt.h>
#include <stdio.h>

void usage(char* command) {
    printf("Usage: %s [option]... rom_image\n", command);
    puts("Pickle risc emulator");
    puts("");
    puts("Options:");
    puts("	-h, --help	Print this message");
}

bool run(const char* romImageFilename) {
    bool ret = false;
    struct cpu_state state;
    cpu_state_init(&state);

    struct dev_memory rom;
    if (!dev_memory_init_ihex(&rom, 0x000000, romImageFilename, /* writable = */ false, &state))
        goto deinit_cpu;

    struct dev_memory ram;
    if (!dev_memory_init_uninitialized(&ram, 0x800000, 0x100000 /* = 1MWord */, /* writable = */ true, &state))
        goto deinit_rom;

    struct dev_uart uart;
    if (!dev_uart_init(&uart, 0x400000, &state))
        goto deinit_ram;

    cpu_state_reset(&state);

    while (true) {
        if (!dev_uart_update(&uart))
            goto deinit_uart;

        if (cpu_state_step(&state))
            break; // Emulator trap encountered
    }

    ret = true;

deinit_uart:
    dev_uart_deinit(&uart);
deinit_ram:
    dev_memory_deinit(&ram);
deinit_rom:
    dev_memory_deinit(&rom);
deinit_cpu:
    cpu_state_deinit(&state);

    return ret;
}

int main(int argc, char** argv) {
    bool verboseFlag = false;

    struct option longOptions[] = {
        {"help", no_argument, 0, 'h'},
        {0, 0, 0, 0}
    };

    while (true) {
        int c = getopt_long(argc, argv, "h:", longOptions, NULL);

        if (c == -1)
            break;

        switch (c) {
        case 0:
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

    if (inputCount != 1) {
        fprintf(stderr, "%s: exactly one ROM image file must be specified\n", argv[0]);
        return EXIT_FAILURE;
    }

    if (run(inputs[0]))
        return EXIT_SUCCESS;
    else
        return EXIT_FAILURE;
}

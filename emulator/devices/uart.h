#pragma once
#include "../cpu_state.h"
#include <stdio.h>

#define DEV_UART_FIFO_SIZE 8

struct dev_uart {
    char readFifo[DEV_UART_FIFO_SIZE];
    word_t readFifoFirst, readFifoLast;
    // Write FIFO is not emulated, we always finish writing immediately.

    int mappingHandle;
    struct cpu_state *cpuState;
};

bool dev_uart_init(
    struct dev_uart *uart,
    physical_address_t mappingStart,
    struct cpu_state *cpuState
);

void dev_uart_deinit(struct dev_uart *uart);

bool dev_uart_update(struct dev_uart *uart);

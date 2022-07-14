#include "uart.h"

#define READ_FIFO_COUNT_OFFSET 0
#define WRITE_FIFO_COUNT_OFFSET 1
#define VALUE_OFFSET 2


static word_t read_fifo_items(struct dev_uart *uart) {
    if (uart->readFifoLast >= uart->readFifoFirst)
        return uart->readFifoLast - uart->readFifoFirst;
    else
        return DEV_UART_FIFO_SIZE - uart->readFifoFirst + uart->readFifoLast;
}

static word_t read_byte(struct dev_uart *uart) {
    word_t ret = uart->readFifo[uart->readFifoFirst];
    uart->readFifoFirst += 1;
    if (uart->readFifoFirst >= DEV_UART_FIFO_SIZE)
        uart->readFifoFirst = 0;

    return ret;
}

static void write_byte(struct dev_uart *uart, word_t c) {
    (void)uart;
    putchar(c);
}

static uint16_t read_fun(void *uartVoidstar, physical_offset_t offset) {
    struct dev_uart *uart = uartVoidstar;
    switch (offset) {
    case READ_FIFO_COUNT_OFFSET:
        return read_fifo_items(uart);
    case WRITE_FIFO_COUNT_OFFSET:
        return 0; // Write FIFO is always empty
    case VALUE_OFFSET:
        return read_byte(uart);
    default:
        return 0;
        // TODO: Trap somehow
    }
}

static void write_fun(void *uartVoidstar, physical_offset_t offset, word_t value) {
    struct dev_uart *uart = uartVoidstar;
    if (offset == VALUE_OFFSET)
        write_byte(uart, value & 0xff);
    // else
        // TODO: Trap somehow
}


bool dev_uart_init(
    struct dev_uart *uart,
    physical_address_t mappingStart,
    struct cpu_state *cpuState
) {
    uart->readFifoFirst = 0;
    uart->readFifoLast = 0;

    struct memory_mapping mapping = {
        .start = mappingStart,
        .end = mappingStart + 4,
        .read = read_fun,
        .write = write_fun,
        .mappingData = uart,
        .mappingId = -1 // Will be set when adding the mapping
    };
    uart->mappingHandle = cpu_state_add_memory_mapping(cpuState, mapping);
    return uart->mappingHandle >= 0;
}

void dev_uart_deinit(struct dev_uart *uart) {
    if (uart->mappingHandle >= 0)
        cpu_state_remove_memory_mapping(uart->cpuState, uart->mappingHandle);
}

bool dev_uart_update(struct dev_uart *uart) {
    (void)uart;
    // TODO: Read from terminal
    return true;
}

# Pickle RISC 16bit CPU

This document contains notes about the hardware design that didn't fit anywhere else.
As usual this is incomplete and slightly outdated.

## Basic design
- 16bit
- Microcoded
- Most instructions take only single clock cycle
    - Instructions with memory access take two
- word addressable memory only!
    - 8bit access emulated in SW, helper instructions
      - `pack` (build word from bytes)
      - `bcmp` (byte-wise compare)
      - `shr8` (shift right by 8 bits)
- 15 general purpose registers R1-R15
- r0 is hardware zero register
- Two stage pipeline
    - Decode, Execute
    - Fetching next instruction is done as part of execute
    - Delay slot after all branch and jump instructions
- Only one status bit: `C`arry
- Control registers (Needs work!)
    - CpuStatus
        - RW
        - Contains:
            - 1b interrupt enabled flag
            - 1b privileged mode flag
                - Controls if privileged instructions are enabled
            - 1b MMU enabled flag
                - If disabled, MMU maps program memory pages 1:1 to frames at the beginning of memory, data pages to frames from in upper half of memory.
            - 6b Context ID
                - Used as a part of virtual page address, equivalent to process ID (with &lt; 64 processes)
    - NextCpuStatus
        - RW
        - Saved CPU status after interrupt
        - New CPU status to be set with ctxsw
    - NextPc
        - RW
        - Saved PC after interrupt
        - New PC to jump to with `ctxsw`
    - IntCause
        - RO
        - Bitmask of interrupt flags (!?!?!?!?)
    - MMUAddr: Virtual page address
        - RW
        - for storing MMU records
        - set during page failure
    - MMUData
        - WO
        - triggers the MMU write at given MMUAddr
    - Display
        - WO
        - Drives a 7-segment display on the front panel
        - 1:1 bit to segment mapping (= no decoder, must be done in SW)
- Separate virtual address spaces for data / code
    - To acces code from a process, the OS must map the memory as data
    - Exception is the ldp (load from program memory) instruction that allows each process to read its program space freely
- Interrupts
- System instructions:
    - `syscall`
        - Causes software interrupt
        - pass immediate value into `IntCause`
            - Quickly distinguish what's necessary in interrupt handler
                - syscall, vs IPC call, vs breakpoint, ...
    - `break`
        - Stop emulator
        - Switch physical CPU into single step mode
            - must be enabled by a physical switch?

## Instruction set weirdness
Compromises have been made :)

- No jumps with immediate values
  - This saves encoding space, but mostly allows us to fetch next instruction after jump without adder delay and without adding another address adder to a different stage.
  - Near jumps can be done in two cycles using `ldpc` (load program counter with offset) and then absolute jump.
  - Tight loops will benefit from preloading jump targets to a register.
- The architecture has something of a "zero page" with faster access
  - Globals within first 256B of memory can be accessed using just `ldi` for address load
    - `ldi addr, 0x17; ld dest, addr + 0`
  - Same for functions -- jumps to first 256 bytes are as fast as relative jumps
    - `ldi addr, 0xa5; jal r0, addr`
- No pop
  - Pop requires two registers to be written.
  - Repeated pops (eg. restoring state after interrupt) can be worked around by batching `ld` with offset followed by a single add. (`ld r1, sp; ld r2,sp+1; ld r3, sp+2; addi sp, sp, 3`)
- `stinc` instead of push
  - This is strictly stronger than push (`push sp, v` = `stinc sp-1, v`)
  - Not sure if arguments other than +-1 are useful, but this instruction is just "we can do it for (almost) free, so why not".
- On wishlist -- slightly weird, but potentially useful:
    - Microcoded multi cycle `memcpy` instruction
      - Probably could run in 2 * N + 1 cycles to copy N words of memory, that's as fast as a dedicated DMA device could run
      - Dedicated DMA would also need to block the CPU (because instruction fetches)
    - Three cycle long atomic swap memory and register instruction
      - Not strictly necessary, because OS can do atomic operations through a syscall, but this would allow us to play with fast futex implementations.

## Microcode ROM
### Incoming signals
(goal is 13 (= 8k ROM), or 15 (= 32k ROM))
- 8 bits from instruction
- 1 bit interrupt pending
- 1 bit kernel mode
- 2 bits condition flags
    - ALU A is zero
    - C
- 2 bits Microcode FSM state

Total 14

### Outgoing control lines
(goal is as small as possible multiple of 8; needs more work)


TODO

- 2 bits Microcode FSM next state

## Memory
- 16 bit-addressable memory (Byte level access emulated in SW)
- Separate data and program segments
- Virtual address format: `CC CCCC S AAAA AA | AA AAAA AAAA`
    - `C` - 6bit context ID (from control register)
    - `S` - segment (0 = data segment, 1 = program segment)
    - `A` - 16bit address
- MMU
    - Built out of two 8k * 8b SRAM ICs
    - Record format: `RWFF FFFF FFFF FFFF`
        - `R` - Read allowed
        - `W` - Write allowed
        - `F` - Frame address (14b)
    - Software page fault handling
        - raises interrupt on access violation
    - 10b page size -> 1kWord = 2kB pages
    - 24b physical address -> 16MWord physical address space
        - 22bit ROM address space
            - 8kWords used (pair of 8k * 8b ROM chips)
        - 22bit device address space
        - 23bit RAM address space
            - max 8MWord = 16MB RAM

## Peripherials wishlist
- UART
- RTC
- Storage
    - SD card using SPI interface?
- [Graphics](notes/graphics.md)
- Network card
    - Probably using the Raspbery PI board (same as with the graphics)
    - W5500 module?
- Hardware multiplier card
    - 16b x 16b -> 32b
    - memory mapped
    - https://www.youtube.com/watch?v=M8dk0JpkrbY

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
    - Display
        - `0`
        - WO
        - Value written here is displayed on the front panel
    - CpuStatus
        - `1`
        - RW
        - Contains:
            - 1b interrupt enabled flag
            - 1b privileged mode flag
                - Controls if privileged instructions are enabled
            - 1b MMU enabled flag
                - If disabled, MMU maps program memory pages 1:1 to frames at the beginning of memory, data pages to frames from in upper half of memory.
    - ContextID
        - `2`
        - WO
        - 6bit
        - Used as a part of virtual page address
            - Equivalent to process ID (with &lt; 64 processes)
    - IntCause
        - `3`
        - RO
        - Cause of currently processed interrupt
    - IntBase
        - `4`
        - WO
        - Where to jump on interrupt
    - IntPc
        - `5`
        - RW
        - Saved program counter after interrupt
        - New program counter used by RETI
    - MMUAddr: Virtual page address
        - `6`
        - RW
        - for storing MMU records
        - set during page failure
    - MMUData
        - `7`
        - WO
        - triggers the MMU write at given MMUAddr
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
- No push
  - We could do address increment in the first execution cycle, in parallel with next instruction fetch, then load the data in the second cycle.
  - One possible version would have RR4 encoding same as `ld` and `st`, would add the immediate.
    - For this we don't have a free 4bit opcode left
  - Other possible version would support only -1 (or also +1, possibly) and take one or two RR instruction slots.
      - I skimped on a bus driver for generating +-1 on the ALU A bus.
- No pop
  - More fundamental problem than push, pop requires two registers to be written.


## Microcode ROM
### Incoming signals
(goal is 13 (= 8k ROM), or 15 (= 32k ROM))
- 8 bits from instruction
- 1 bit interrupt pending
- 1 bit kernel mode
- 2 bits condition flags
    - ALU A is zero
    - C

Total 12

### Outgoing control lines
(goal is as small as possible multiple of 8; needs more work)

TODO

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

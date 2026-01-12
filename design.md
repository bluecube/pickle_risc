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
    - R0 is hardware zero register
- Two stage pipeline
    - Decode, Execute
    - Fetching next instruction is done as part of execute
    - Delay slot after all branch and jump instructions
- Only one status bit: `C`arry
- Control registers (Needs work!)
    - Display
        - `0`
        - RW
        - Value written here is displayed on the front panel
    - CpuStatus
        - `1`
        - RW
        - Contains:
            - 1b interrupt enabled flag
            - 1b kernel mode flag
            - 1b MMU enabled flag
            - 1b sleep bit
            - ?b frequency selection (?)
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
    - Exception is the ldp (load from program mmemory) instruction that allows each process to read its program space freely
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
- Loading 16bit immediates takes three cycles
  - This is a bit painful :-)
  - The best sequence for loading a full 16bit immediate is:
    - `ldi rd, hi8(value) + (value & 0x80 >> 7)`
      - Load the high byte to low byte of rd, add compensation for sign extended addi that comes later
    - `pack rd, rd, r0`
      - Shift the destination register up by a byte, fill with zero
    - `addi rd, lo8(value)`
      - Add the low byte
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
- 3 bits condition flags
    - ALU A is zero
    - C
    - L

Total 13

### Outgoing control lines
(goal is as small as possible multiple of 8; needs more work)

- 1b: ALU A bus source
- 2b: ALU B bus source
    - imm4 vs imm8
    - imm vs register B 
- 5b: Result bus source
    - 3b ALU result
    - 2b ALU vs memory vs Pc vs Control register
- 1b: Addres base bus source
    - PC vs register B
- 3b: Addres offset bus source
    - 0 / 1
    - imm4(A) vs imm8(AB) vs imm8(BD) vs (0/1)
- 1b: load register D
- 1b: load Pc
- 1b: Load control register

- 1b: Mem read
- 1b: Mem write

- 1b: End instruction

- 1b: Write interrupt ID into cause register, clear pending interrupt flag
- 1b: Write 6bits from immediate to upper 8bits of cause register

Total 20

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
- Network card
    - W5500 module?

## Parts
- 74LVC16374 - 16-bit edge-triggered D-type flip-flop; 5 V tolerant; 3-state


## !!!
- SN54AHCT574 requires 1.5ns hold time on data change
    - Is it Ok to rely on propagation delays of other circuitry for single cycle read-modify-write?
        - Verify with the fastest possible roundtrip path
    - Maybe the output delay of the register itself is enough?
        - Probably yes: min output delay is 1ns, there probably won't be any path faster than 0.5ns

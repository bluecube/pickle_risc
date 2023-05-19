# Pickle RISC 16bit CPU

This document contains notes about the hardware design that didn't fit anywhere else.
As usual this is incomplete and slightly outdated.

## Basic design
- 16bit
- microcoded
- most instructions take only single clock cycle
- word addressable memory only!
    - 8bit access emulated in SW
- 15 general purpose registers R1-R7
    - R0 is hardware zero register
- Control registers
    - ALUStatus
        - `0`
        - RW
        - Accessible from user mode
        - Contains:
            - 1b carry flag
            - 1b zero flag
            - 1b negative flag
            - 1b overflow flag
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
- Separate instruction virtual address space
    - Still accessible through memory mapping
- Interrupts
- System instructions:
    - Syscall instruction
        - Causes software interrupt
        - pass 6bit immediate value into high 8 bits of `IntCause`
            - Quickly distinguish what's necessary in interrupt handler
                - syscall, vs IPC call, vs breakpoint, ...
    - Break instruction
        - Stop emulator
        - Switch physical CPU into single step mode
            - must be enabled by a physical switch?
- 3 stage pipeline
    1. fetch
    2. decode
        - dominated by 150ns microcode ROM access time
    3. execute
        - dominated by 4 * 45ns 4bit adder propagation delay

    - Theoretically about 5MHz max clock speed?


## Microcode ROM
### Incoming signals
(goal is 13 (= 8k ROM), or 15 (= 32k ROM))
- 8 bits from instruction
- 1 bit interrupt pending
- 1 bit kernel mode
- 3 bits condition flags

Total 13

### Outgoing control lines (TODO: Outdated)
(goal is as small as possible multiple of 8)

- 2b: Left bus source
    - `0->left`: 0
    - `f1->left`: GPR: instruction field 1
    - `f3->left`: GPR: instruction field 3
    - `pc->left`: Pc
- 2b: Right bus source
    - `f2->right`: GPR: instruction field 2
    - `f4->right`: Control register: instruction field 4
    - `f5->right`: Immediate value: instruction field 5
- 4b: Result bus source
    - ALU
        - `alu_add->result`
        - `alu_addc->result`
        - `alu_sub->result`
        - `alu_subc->result`
        - `alu_and->result`
        - `alu_or->result`
        - `alu_nor->result`
        - `alu_xor->result`
        - `alu_andshr->result`
        - `alu_andshra->result`
        - `alu_andshrc->result`
        - `alu_bswp->result`
    - `mem_data->result`: Memory data
- 1b: Address base bus source
    - `right->address`: Right bus
    - `pc->address`" Pc
- 2b: Address offset bus source
    - `0->addr_offset`: 0
    - `1->addr_offset`: 1
    - `f6->addr_offset`: Immediate value: instruction field 6 (load/store)
    - `f7->addr_offset`: Immediate value: Instruction field 7 (rjmp)
- 1b: `f4_override` Override control register selection to value 0 (instruction field 4)
- 1b: `store_f4`: Load control register: instruction field 4
- 1b: `store_f3`: Load GPR: instruction field 3
- 1b: `store_pc`: Load PC
- 1b: Memory data bus source
    - `left->mem_data`: Assign from left bus
    - `read->mem_data`: Memory read
- 1b: `mem_write`: Memory write
- 1b: `end_instruction`: Reset microprogram counter, clock the decoded μcode latch
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
- 2x UART
    - one for console, one for networking
- RTC
- Storage
    - SD card using SPI interface?

## Parts
- 74LVC16374 - 16-bit edge-triggered D-type flip-flop; 5 V tolerant; 3-state


## !!!
- SN54AHCT574 requires 1.5ns hold time on data change
    - Is it Ok to rely on propagation delays of other circuitry for single cycle read-modify-write?
        - Verify with the fastest possible roundtrip path
    - Maybe the output delay of the register itself is enough?
        - Probably yes: min output delay is 1ns, there probably won't be any path faster than 0.5ns

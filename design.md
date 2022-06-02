# Pickle RISC 16bit CPU

## Basic design
- 16bit
- microcoded
- mostly RISC, but some complex instructions
- 16bit addressable memory only!
    - 8bit access emulated in SW
- 8 general purpose registers R0-R7
- Control registers
    - ALU Status
        - RW
        - Can accessed from user mode
        - Contains:
            - carry flag
            - zero flag
            - negative flag
    - Tmp1
        - RW
        - Used as temporary storage in interrupt handlers
        - Accessible to user-mode code, but not saved during interrupts (=> unusable)
        - Clobbered by some complex instructions
            - Cannot be used by the kernel to store data between context switches
    - Tmp2
        - RW
        - Used as temporary storage in interrupt handlers
    - ContextID
        - WO
        - 6bit
        - equivalent to process ID (with &lt; 64 processes)
        - Context ID of 0 for kernel mode and disabled interrupts, otherwise user mode
        - used as a part of virtual page address
        - double buffered, only gets applied by RETI instruction
    - IntCause
        - RO
        - Cause of currently processed interrupt
    - IntPc
        - Saved program counter after interrupt
        - New program counter used by RETI
    - MMUAddr: Virtual page address
        - RW
        - for storing MMU records
        - set during page failure
    - MMURecord
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
- Jump
    - There's no jump and link instruction, instead use `addipc Rlink, 1` with normal jump instruction.
- 2 stage pipeline
    1. fetch + decode
        - dominated by 150ns microcode ROM access time
    2. execute
        - dominated by 4 * 45ns 4bit adder propagation delay

    - Theoretically about 5MHz max clock speed?


## Instruction format

### Fields

<table>
<tr>
    <th>Field number</th>
    <th>15</th><th>14</th>
    <th>13</th><th>12</th>
    <th>11</th><th>10</th>
    <th>9</th><th>8</th>
    <th>7</th><th>6</th>
    <th>5</th><th>4</th>
    <th>3</th><th>2</th>
    <th>1</th><th>0</th>
</tr>
<tr><th>0</th><td colspan="7">opcode</td><td colspan="9"></td></tr>
<tr><th>1</th><td colspan="7"></td><td colspan="3">register ID<br>assert left bus</td><td colspan="6"></td></tr>
<tr><th>2</th><td colspan="10"></td><td colspan="3">register ID<br>assert right bus</td><td colspan="3"></td></tr>
<tr><th>3</th><td colspan="13"></td><td colspan="3">register ID<br>assert left bus, load from result bus</td></tr>
<tr><th>4</th><td colspan="5"></td><td colspan="3">control register ID<br>assert right bus, load from result bus</td><td colspan="8"></td></tr>
<tr><th>5</th><td colspan="5"></td><td colspan="8">immediate value<br>assert right bus</td><td colspan="3"></td></tr>
<tr><th>6</th><td colspan="3"></td><td colspan="7">immediate value (load/store)<br>add to address</td><td colspan="6"></td></tr>
<tr><th>7</th><td colspan="7"></td><td colspan="9">immediate value (rjmp)<br>add to address</td></tr>
</table>

### Opcodes

<table>
<tr>
    <th>00</th>
    <td colspan="3">Immediate operations</td>
</tr>
<tr>
    <th rowspan="2">01</th>
    <th>0</th>
    <td colspan="2">Basic 3 operand operations</td>
</tr>
<tr>
    <th>1</th>
    <td colspan="2">Complex 3 operand operations</td>
</tr>
<tr>
    <th>10</th>
    <td colspan="3">Load / store</td>
</tr>
<tr>
    <th rowspan="3">11</th>
    <th>0</th>
    <td colspan="2">Jump</td>
</tr>
<tr>
    <th rowspan="2">1</th>
    <th>0</th>
    <td>Control registers</td>
</tr>
<tr>
    <th>1</th>
    <td>System operations</td>
</tr>
</table>

## Microcode ROM
### Incoming signals
(goal is 13 (= 8k ROM), or 15 (= 32k ROM))
- 7 bits from instruction
- 3 bits from microprogram counter
- 1 bit interrupt pending
- 1 bit kernel mode
- 3 bits condition flags

Total 15

### Outgoing control lines
(goal is as small as possible multiple of 8)

- 2b: Left bus source
    - GPR: instruction field 1
    - GPR: instruction field 3
    - Pc
- 2b: Right bus source
    - GPR: instruction field 2
    - Immediate value: instruction field 5
    - Control register: instruction field 4
- 1b: Result bus source
    - ALU
    - Memory read
- 1b: Address base bus source
    - Right bus
    - Pc
- 2b: Address offset bus source
    - 0
    - +1
    - Immediate value: instruction field 6 (load/store)
    - Immediate value: Instruction field 7 (rjmp)
- 1b: Override control register selection (instruction field 4)
- 1b: Load control register: instruction field 4
- 1b: Load GPR: instruction field 3
- 1b: Load PC
- 1b: Memory write
- 5b: ALU control (TODO, 5b is the lower bound)
- 1b: Reset microprogram counter, clock the decoded μcode latch
- 1b: Write interrupt ID into cause register, clear pending interrupt flag
- 1b: Write 6bits from immediate to upper 8bits of cause register

Total 21

#### TODO
- How is PC increment and "pipelining" handled?
- ALU control
- How to store interrupt cause from SW interrupt?

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
    - Hardcoded page:
        - Context = 0, Segment = 1, Address = 0x0000 - 0x0FFF) -> boot EEPROM, read only
            - Need to map further pages manually from bootloader
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
- GPU
    - Not too interested in building one, but it would be nice to have one
    - Perhaps we could cheat and connect RaspberryPI to the memory bus and use it as a GPU?
- Sound
    - Same as GPU, maximum cheating

## Parts
- 74LVC16374 - 16-bit edge-triggered D-type flip-flop; 5 V tolerant; 3-state


## !!!
- SN54AHCT574 requires 1.5ns hold time on data change
    - Is it Ok to rely on propagation delays of other circuitry for single cycle read-modify-write?
        - Verify with the fastest possible roundtrip path
    - Maybe the output delay of the register itself is enough?
        - Probably yes: min output delay is 1ns, there probably won't be any path faster than 0.5ns

# Pickle RISC 16bit CPU

Notes regarding the ISA of Pickle RISC DIY 16bit CPU

## Goals
- Fun!
- Multitasking OS
- Serve web site about itself
    - SLIP over UART?
- Serial console interface
- Potentially Games
- Run at ~2MHz clock
- Most instruction single cycle

## Links
### Inspiration
- Magic-1: http://www.homebrewcpu.com/
- James Sharman's pipelined 8bit CPU: https://www.youtube.com/c/weirdboyjim/videos

### CPU design / instruction set
- MUPS/16, a nice 16bit CPU with similar goals/design: http://mups16.net/pages/overview.html#overview
- MIPS Architecture
- ARM Thumb Instruction set

### The rest
- RISC-V: https://riscv.org/wp-content/uploads/2017/05/riscv-spec-v2.2.pdf
- Simple almost C-like language: https://incoherency.co.uk/blog/stories/slang.html
- Logisim evolution
- Nandgame ALU https://nandgame.com/
    > The ALU (Arithmetic/Logic Unit) performs one or more operations on two input values X and Y.
    >
    > The six flags select what operations to perform. Each flag trigger an operation when the flag is 1:
    > zx	Use 0 for X
    > nx	Invert X
    > zy	Use 0 for Y
    > ny	Invert Y
    > f	selects an operation:
    > 0: output is X AND Y
    > 1: output is X + Y
    > no	Invert output
    >
    > The flags can be combined and the specified order is significant.
    >
    > For example, if both zx and nx is 1, then X is inverted 0.

### Parts
- 74LVC16374 - 16-bit edge-triggered D-type flip-flop; 5 V tolerant; 3-state

## Basic design
- 16bit
- microcoded
- mostly RISC, but some complex instructions
- 16bit addressable memory only!
    - 8bit access emulated in SW
- 8 general purpose registers R0-R7
    - R7: General purpose / link register
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
    - Three syscall instructions
    - Causes software interrupt with three different causes
        - Intended use:
            1. Syscall
            2. Debugger breakpoint
            3. IPC call
    - Break instruction
        - Stop emulator
        - Switch physical CPU into single step mode
            - must be enabled by a physical switch?

## Instruction format

<table>
<tr>
    <th>0</th><th>1</th>
    <th>2</th><th>3</th>
    <th>4</th><th>5</th>
    <th>6</th><th>7</th>
    <th>8</th><th>9</th>
    <th>10</th><th>11</th>
    <th>12</th><th>13</th>
    <th>14</th><th>15</th>
</tr>
<tr><td colspan="7">opcode</td><td colspan="9"></td></tr>
<tr><td colspan="7"></td><td colspan="3">register ID<br>assert left bus</td><td colspan="6"></td></tr>
<tr><td colspan="10"></td><td colspan="3">register ID<br>assert right bus</td><td colspan="3"></td></tr>
<tr><td colspan="13"></td><td colspan="3">register ID<br>assert right bus, load from result bus</td></tr>
<tr><td colspan="5"></td><td colspan="3">register ID<br>assert right bus, load from result bus</td><td colspan="8"></td></tr>
<tr><td colspan="5"></td><td colspan="3">control register ID<br>assert right bus, load from result bus</td><td colspan="8"></td></tr>
<tr><td colspan="8"></td><td colspan="8">immediate value<br>assert right bus, add to address</td></tr>
<tr><td colspan="3"></td><td colspan="7">immediate value<br>add to address</td><td colspan="6"></td></tr>
</table>

## Microcode ROM
### Incoming signals
(goal is 13 (= 8k ROM), or 15 (= 32k ROM))
- 7 bits from instruction
- 3 bits from microprogram counter
- 1 bit interrupt pending
- 1 bit kernel mode
- 3 bits condition flags

Total 14

### Outgoing signals
(goal is as small as possible multiple of 8)
- 4 bits: General purpose register assert / load flags
- 1 bits: Control register read
- 1 bit: Control register write
- 1 bit: Control register id override (set to 111)

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

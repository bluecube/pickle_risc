# Pickle RISC 16bit CPU

Notes regarding the ISA of Pickle RISC DIY 16bit CPU

## Goals
- Fun!
- Multitasking OS
- Serve web site about itself
    - SLIP over UART?
- Serial console interface
- Potentially Games

## Links
### Other similar CPUs
- Magic-1: http://www.homebrewcpu.com/
- James Sharman's pipelined 8bit CPU: https://www.youtube.com/c/weirdboyjim/videos
- MUPS/16, a nice 16bit CPU with similar goals/design: http://mups16.net/pages/overview.html#overview

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
- 16bit only memory!
    - 8bit access emulated in SW
        - We should add instructions that will make standard string operations fast
            - Basically SIMD?
- 3-operand architecture
- 8 general purpose registers:
    - R0: zero register (always reads as zero, written value is ignored)
    - R1-R6: general purpose (R6 used as stack pointer?)
    - R7: General purpose / link register
- Separate instruction virtual address space
    - Still accessible through memory mapping
- MMU (getting fancy in here!)
- Interrupts

## Instruction set

### ALU op:
Rd = Rl <op> Rr

000d ddll lrrr pppp

### load
Rd = [Rl + imm]
imm is 4 bit signed

001d ddll lxxx iiii

### store
tmp = Rl + imm
[tmp] = Rr
Rd = tmp
imm is 4 bit signed

010d ddll lrrr iiii

### load immediate
if h:
    Rd = imm << 7
else
    Rd = imm
imm is 9bit unsigned

011d ddhi iiii iiii

### jump
if (Rr != 0) ^ n:
    if link: R8 = PC
    PC = Rl + imm
imm is 4 bit signed
100k nxll lrrr iiii

### jump relative
if (Rr != 0) ^ n:
    if link: R8 = PC
    PC = PC + imm
imm is 4 bit signed
101k nxxx xrrr iiii


### Read control register
Rd = Cc

110d ddcc cxxx xxxx

### Write control register
Cc = Rr

111x xxcc crrr xxxx

### Syscall
<Interrupt>
IntId = imm

111x xx00 0000 iiii


## ALU operations
- Rd = Rl + Rr
- Rd = Rl + Rr + Carry
- Rd = Rl - Rr
- Rd = Rl - Rr - Carry

- Rd = Rl & Rr
- Rd = Rl | Rr
- Rd = !(Rl ^ Rr)

- Rd = Ra >> 1, logical
- Rd = Ra >> 1, arithmetic
- Rd = Ra >> 1, carry

- Rd = Rl > Rr, unsingned
- Rd = Rl >= Rr, unsigned
- Rd = Rl > Rr, signed
- Rd = Rl >= Rr, signed

- Shuffle bytes: Rd = Ra >> 8 | Rb << 8
- Byte equality:
    - Sets output bit 0 if lower byte of Ra equals lower byte of Rb
    - Sets output bit 1 if upper byte of Ra equals lower byte of Rb
    - Sets output bit 2 if lower byte of Ra equals upper byte of Rb
    - Sets output bit 3 if upper byte of Ra equals upper byte of Rb

### Not included:
- shl: Can be done as Rx + Rx
- ==: Can be done using xnor
- <, <=: Flip operands
- unary -: R0 - Rx
- xor:
    - Not very frequent (?)
    - Can be emulated using Rd = ~(Ra ^ Rb); Rd = ~(Rd ^ 0);
    - Or can some logic be converted to use the xnor anyway?

### ?
- How to use second operand of >>?
- Helper functionality for mul/div?

## Memory model:
    16 bit-addressable memory (Byte level access emulated in SW)
    Separate data and program segments
    Virtual address format: CC CCCC S AAAA A | AAA AAAA AAAA
        C - 6bit context ID (from control register)
        S - segment (0 = data segment, 1 = program segment)
        A - 16bit address
    MMU:
        Built out of two 8k * 8b SRAM ICs -- only half used :-(
        Record format: RWFF FFFF FFFF FFFF
            R - Read allowed
            W - Write allowed
            F - Frame address (14b)
        Hardcoded pages:
            Context = 0, Segment = 0, Address = 0x0000 - 0x0FFF) -> MMU SRAM itself
                - Use as stack space during early bootup
             Context = 0, Segment = 1, Address = 0x0000 - 0x0FFF) -> boot EEPROM, read only
                - Need to map further pages manually
                - This would solve boot, but would force us to do interrupts in other location
        Software page fault handling
            raises interrupt on access violation
    11b page size -> 2kWord = 4kB pages
    25b physical address -> 64MB physical address space
        24bit device address space
        23bit RAM address space
            max 8kWord = 16MB RAM
        23bit ROM address space
            2kWords used?

## Peripherials wishlist
- 2x UART
    - one for console, one for networking
- RTC
- GPU
    - Not too interested in building one, but it would be nice to have one
    - Perhaps we could cheat and connect RaspberryPI to the memory bus and use it as a GPU?
- Sound
    - Same as GPU, maximum cheating

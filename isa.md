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
- 16bit addressable memory!
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

| Opcode	| Instruction				|
|-----------|---------------------------|
| `00`		| ALU immediate				|
| `010`		| ALU register				|
| `011`		| Load immediate			|
| `100`		| Load						|
| `101`		| Store						|
| `1100`	| Jump relative				|
| `1101`	| Jump						|
| `1110`	| Read control register		|
| `11110`	| Write control register	|
| `11111`	| Syscall					|

### ALU immediate:
Rd = Rl <ALU_op> imm
imm is 4bit unsigned

00ia aaal llii iddd

### ALU register:
Rd = Rl <ALU_op> Rr

010a aaal llrr rddd

### load immediate
if h:
    Rd = imm << 7
else
    Rd = imm
imm is 9bit unsigned

011h iiii iiii iddd

### load
Rd = [Rl + imm]
imm is 4 bit signed

100i iiil llxx xddd

### store
tmp = Rl + imm
[tmp] = Rr
Rd = tmp
imm is 4 bit signed

101i iiil llrr rddd

### jump relative
if (Rl != 0 || (Carry && CarryFlag)) ^ NegateFlag:
    PC = PC + imm
imm is 7 bit signed

1100 ncil llii iiii

### jump
if (Rl != 0 || (Carry && CarryFlag)) ^ NegateFlag:
    if LinkFlag:
        R8 = PC
    PC = Rr

1101 nckl llrr rxxx

### Read control register
Rd = Cc

1110 xxxx xxcc cddd

### Write control register
Cc = Rl

1111 0xxl llcc cxxx

### Syscall
<Interrupt>
IntId = imm
imm is 7 bit unsigned

1111 1xix xxii iiii


## ALU operations
- Rd = Rl + Rr
- Rd = Rl + Rr + Carry
- Rd = Rl - Rr
- Rd = Rl - Rr - Carry

- Rd = Rl & Rr
- Rd = Rl | Rr
- Rd = !(Rl | Rr)
- Rd = Rl ^ Rr

- Rd = Ra >> 1, logical
- Rd = Ra >> 1, arithmetic
- Rd = Ra >> 1, carry

- Rd = Rl > Rr, unsingned
- Rd = Rl > Rr, signed

- Shuffle bytes: Rd = Ra >> 8 | Rb << 8
- Byte equality:
    - Sets output bit 0 if lower byte of Ra equals lower byte of Rb
    - Sets output bit 1 if upper byte of Ra equals lower byte of Rb
    - Sets output bit 2 if lower byte of Ra equals upper byte of Rb
    - Sets output bit 3 if upper byte of Ra equals upper byte of Rb

### Not included:
- shl: Can be done as Rx + Rx
- ==: Can be done using xnor
- <: Flip operands
- >=, <=: Use negative conditions
- unary -: R0 - Rx

### ?
- How to use second operand of >>?
    - Maybe convert it to midpoint?
- Helper functionality for mul/div?

## Memory model:
- 16 bit-addressable memory (Byte level access emulated in SW)
- Separate data and program segments
- Virtual address format: CC CCCC S AAAA AA | AA AAAA AAAA
    - C - 6bit context ID (from control register)
    - S - segment (0 = data segment, 1 = program segment)
    - A - 16bit address
- MMU
    - Built out of two 8k * 8b SRAM ICs
    - Record format: RWFF FFFF FFFF FFFF
        - R - Read allowed
        - W - Write allowed
        - F - Frame address (14b)
    - Hardcoded page:
        - Context = 0, Segment = 1, Address = 0x0000 - 0x0FFF) -> boot EEPROM, read only
            - Need to map further pages manually from bootloader
            - This would solve boot, but would force us to do interrupts in other location
    - Software page fault handling
        - raises interrupt on access violation
    - 10b page size -> 1kWord = 2kB pages
    - 24b physical address -> 16MWord physical address space
        - 22bit ROM address space
            - 2kWords used?
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

# Pickle RISC 16bit CPU

... so I would like to build a computer, this time a little closer to "from scratch".

## Goals
- Fun!
- Can run a multitasking OS
- Serve web site about itself (like Magic-1 does)
    - SLIP over UART?
- Serial console interface
- Potentially Games
- Run at ~2MHz clock
- Most instruction single cycle

## State
- [Instruction set](http://htmlpreview.github.io/?https://github.com/bluecube/pickle_risc/blob/master/instruction_set.html)
    - Basics done, good enough to start writing assembler and simulator
    - Some complex instructions should definitely be added later
    - Needs to be tweaked according to what's needed in SW, limitations in HW design
- HW design
    - Slightly more than a fuzzy idea of how stuff should should go together
    - The [block diagram](block_diagram.svg) shows most of the general idea
- SW
    - MWP assembler is done

## Next steps
- Figure out ALU design
- Figure out MMU design
- Figure out interrupt handling
    - Rough idea and the instructions needed to support it is already done
- Emulator
- Start building the hardware in some kind of HDL

## Links
### Inspiration
- Magic-1: http://www.homebrewcpu.com/
    - I would like my machine to look like Magic-1 and do the stuff that Magic-1 does.
- James Sharman's pipelined 8bit CPU: https://www.youtube.com/c/weirdboyjim/videos
    - Different goals, but just plain awesome :)

### Instruction set
- MIPS Architecture
- ARM Thumb Instruction set

### Peripherials
- Interfacing SDHC card to 6502: https://github.com/gfoot/sdcard6502

### The rest
- MUPS/16, a nice 16bit CPU with similar goals/design: http://mups16.net/pages/overview.html#overview
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

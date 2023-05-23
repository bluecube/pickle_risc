# Pickle RISC 16bit CPU

... so I would like to build a computer, this time a little closer to "from scratch".

## Goals
- Fun!
- Aiming for a poorly defined balance between "easy to build" and "not too slow"
- Can run a multitasking OS
- Serve web site about itself (like Magic-1 does)
    - SLIP over UART?
    - W5500 module?
- Serial console interface
    - No graphics output
- Run at ~2MHz clock
    - 4MHz as a stretch goal :)
- Most instruction single cycle

## State
- [Instruction set](http://htmlpreview.github.io/?https://github.com/bluecube/pickle_risc/blob/master/instruction_set.html)
    - Needs expanding into microcode
    - Needs to be tweaked according to what's needed in SW, limitations in HW design
- HW design
    - Slightly more than a fuzzy idea of how stuff should should go together
    - (OUTDATED) The [block diagram](block_diagram.svg) shows most of the general idea
    - There is a [design document](design.md) that contains some (incomplete) information
- SW
    - Skeleton of a toolchain written in Rust ([toolchain-rs/](toolchain-rs/))
        - Non-functioning assembler
        - Start of an emulator
            - Emulation works per instruction, but behavior is built up from microcode at compile time.
    - [Lisp](notes/lisp.md) interpreter is planned as a shell
        - Probably loosely based on [Make a Lisp](https://github.com/kanaka/mal/blob/master/process/guide.md)
        - Compiler as a stretch goal

## Next steps
This is the high level to do list, roughly in the order of decreasing priorities.
Long term plans (and hopes and dreams) go to the [notes](notes/) directory.

- Figure out details of microcode
- Write microcode for instructions
- Finalize assembler
- Emulator
- Some kind of integration tests for emulator <-> assembler
- Lisp interpreter
- Figure out ALU design
- Figure out MMU design
- Figure out interrupt handling
    - Rough idea and the instructions needed to support it is already done
- Start building the hardware in some kind of HDL

## Links
### Inspiration
- Magic-1: http://www.homebrewcpu.com/
    - Showed me that there is so much magic in this topic.
    - I would like my machine to look like Magic-1 and do the stuff that Magic-1 does.
- Ben Eater's breadboard computer: https://www.youtube.com/playlist?list=PLowKtXNTBypGqImE405J2565dvjafglHU
    - Gave me the idea that I could actually build something like this.
- James Sharman's JAM-1: https://www.youtube.com/c/weirdboyjim/videos
    - Different goals, but just plain awesome :)

### Instruction set
- ARM Thumb Instruction set
- AVR instruction set

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

# Pickle RISC 16bit CPU

... so I would like to build a computer, this time a little closer to "from scratch".

## Goals
- Fun!
- Aiming for a poorly defined balance between "easy to build" and "not too slow"
- Can run a multitasking OS
- Serve web site about itself (like Magic-1 does)
- Serial console interface
- Run at ~2MHz clock
    - 4MHz as a stretch goal :)
- Most instruction single cycle
- Probably RPi as a peripherial device, emulating a [GPU](notes/graphics.md), network card, storage, serial terminal...

## State
- Instruction set
    - Kind of done, still being tweaked to match the HW limitations and to improve proramming experience
- HW design
    - Slightly more than a fuzzy idea of how stuff should should go together
    - The block diagram shows most of the general idea
    - There is a [design document](design.md) that contains some (incomplete) information
- SW
    - Skeleton of a toolchain written in Rust ([toolchain-rs/](toolchain-rs/))
    - [Lisp](notes/lisp.md) interpreter is planned as a shell
        - Probably loosely based on [Make a Lisp](https://github.com/kanaka/mal/blob/master/process/guide.md)
        - Compiler as a stretch goal

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

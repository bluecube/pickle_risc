""" This file defines the instruction set architecture of trilo8bit.

Little endian

Registers
    r0 - 8bit Accumulator
    r1, r2, - 8bit general purpose / 16bit pointer
    r3, r4 - 8bit general purpose / 16bit pointer (with inc store)
    Sp - 16bit stack pointer
    Pc - 16bit program counter - points at the next instruction to be executed
    Int - 16bit interrupt return address / kernel mode flag (Int != 0 => kernel mode)
    Pid - 8bit page / process ID
    Pidl - 8bit latch register for Pid to allow atomic context switches with reti
    Z - 1bit zero flag
    C - 1bit carry flag
    N - 1bit negative flag

Interrupts (4bit):
    0 - None
    1 - serial1
    2 - serial2
    3 - storage
    4 - network1
    5 - network2
    6 - RTC tick
    7 - RTC alarm
    8 - 10 - ...
    11 - DMA finished
    12 - MMU tlb miss
    13 - MMU access violation
    14 - internal - protection fault (call protected instruction from user mode)
    15 - internal - int instruction called


Peripherials (very preliminary):
    Boot ROM - 2kB eeprom, should be RW by kernel
    MMU
        - 2kB pages
        - 8b pid + 5b page number -> 14b physical frame number + 1b W flag + 1b X flag
        - TLB with 2(?) hard coded (boot, MMU control, ...) and 8 dynamic records
        - software page fault handling
        - max 32MB of addressable physical memory. That leaves 16MB for RAM and another 16MB for memory mapped peripherial control.
        - Later: DMA engine with transparent transfers up to 256B
    RAM
    GPU
        - 320x240, 2bit grayscale (or 4 item palette of 3:3:2 RGB colors)
        - double buffered
        - VGA output?
    Serial interface (USB?) + 2 digit 7segment display
    Storage - SD card?
    Network - PPP or SLIP?
    RTC - interrupts at given rate and at set time

Modes:
    run - normal mode, clock is running
        - 1st tick fetch and decode
        - 2nd tick execute
    DMA - clock is running, CPU does not fetch or execute instructions
        - DMA unit has all clock ticks available for transfers.
        - Can only be exited via interrupt
    sleep - clock is stopped
        - Can only be exited via interrupt
"""

instructions = {
    "MOV":     ("0rrr aaaa", [("target", "r"), ("source", "a")], "target (register) <- source"),
    "NAND":    ("0101 aaaa", [("arg", "a")], "R0 <- R0 nand arg, sets status flags based on result"),
    "ADD":     ("0110 aaaa", [("arg", "a")], "R0 <- R0 + arg + C, sets status flags based on result"),
    "SUB":     ("0111 aaaa", [("arg", "a")], "R0 <- R0 - arg - C, sets status flags based on result"),
    "ST":      ("1rrr 0sss", [("target", "s"), ("source", "r")], "target (memory) <- source"),
    "INC":     ("1rrr 1000", [("arg", "r")], "arg <- arg + 1"),
    "DEC":     ("1rrr 1001", [("arg", "r")], "arg <- arg - 1"),
    "CHK":     ("1rrr 1010", [("arg", "r")], "sets status flags based on value of arg"),
    "NEG":     ("1rrr 1011", [("arg", "r")], "arg <- -arg, sets status flags based on result"),
    "SHL":     ("1rrr 1100", [("arg", "r")], "arg <- arg << 1 | C, sets status flags, based on result"),
    "SHR":     ("1rrr 1101", [("arg", "r")], "arg <- arg >> 1 | C << 7, sets status flags, based on result"),
    #"":       ("1rrr 110_", [], "")
    "MOVW":    ("1101 ppqq", [("target", "p"), ("source", "q")], "target (16 bit) <- source (16 bit)"),
    "SKIP1IF": ("1110 0ccc", [("condition", "c")], "If condition: Pc <- Pc + 1"),
    "SKIP2IF": ("1110 1ccc", [("condition", "c")], "If condition: Pc <- Pc + 2"),
    "SETI":    ("1111 0000", [], "Int <- (R3,R4)"),
    "GETI":    ("1111 0001", [], "(R3,R4) <- Int"),
    "SETPID":  ("1111 0010", [], "Pidl <- r0"),
    "RETI":    ("1111 0011", [], "Pc <- Int, Pid <- Pidl, Int <- 0, Pidl <- 0"),
    "CALL":    ("1111 01qq", [("address", "q")], "Pc <-> address"),
    "CLRC":    ("1111 1000", [], "C <- 0"),
    "SETC":    ("1111 1001", [], "C <- 1"),
    "PUSHST":  ("1111 1010", [], "[-Sp] <- (Z, C, N)"),
    "POPST":   ("1111 1011", [], "(Z, C, N) <- [Sp+]"),
    "RJMP":    ("1111 1100", [("offset", "i")], "Pc <- Pc + offset ( = [Pc])"),
    "DMA":     ("1111 1101", [], "Switch into DMA mode."),
    "SLEEP":   ("1111 1110", [], "Switch into sleep mode."),
    "INT":     ("1111 1111", [], "Raise interrupt 15."),
}

R0 = Register("R0")
R1 = Register("R1")
R2 = Register("R2")
R3 = Register("R3")
R4 = Register("R4")


gp_registers = []
regpair_names = "Pc R1 R3 Sp".split()

arg_types = {
    "r": (gp_register_names, "Register"),
    "a": (gp_register_names +
          [""] * 4 +
          ["[{}]".format(regpair) for regpair in regpair_names[1:]] +
          ["[{}.inc]".format(regpair) for regpair in regpair_names],
          "ALU value"),
    "s": ([""] +
          ["[{}]".format(regpair) for regpair in regpair_names[1:]] +
          [""] +
          ["[{}.inc]".format(regpair) for regpair in regpair_names[1:-1]] +
          ["[{}.dec]".format(regpair_names[-1])],
          "Store target"),
    "p": (regpair_names, "Regpair target"),
    "q": (regpair_names, "Regpair source"),
    "c": (["True", "False",
           "C", "not C",
           "Z", "not Z",
           "N", "not N"], "Skip condition")
}

if __name__ == "__main__":
    import pprint
    print("Instructions:")
    pprint.pprint(instructions)
    print()
    print("Argument types:")
    pprint.pprint(arg_types)

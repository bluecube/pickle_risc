""" This file defines the instruction set architecture of trilo8bit.
It's the main source of information for the assembler, microcode program and simulator.

Little endian

Registers
    Tos0 - 8bit top of stack
    Tos1 - 8bit second value on the stack
    A - 16bit pointer
    B - 16bit pointer
    Sp - 16bit stack pointer - points at the third item on the stack, first in memory
    Pc - 16bit program counter - points at the next instruction to be executed
    IPc - 16bit interrupt return address / kernel mode flag (Int != 0 => kernel mode)
    Pid - 8bit page / process ID
    N - 1bit negative flag
    I - 8bit instruction register (not accessible to programmer)
    Tmp - 8bit Temporary register (not accessible to programmer)

Interrupts (4bit):
    0 - serial0 data available
    1 - serial0 write complete
    2 - serial1 available
    3 - serial1 write complete
    4 - RTC_tick
    5 - RTC_alarm
    ...
    14 - MMU access violation
    15 - Software interrupt (syscall)

    TODO: Interrupt masking

    After interrupt the CPU saves current Pc to the IntPc register and jumps to address 0x1000 + Int << 3.
    Interrupts are disabled and 0 is used as a Pid for code access (but not data access) while IntPc is nonzero.
    No state other than Pc is saved during interrupt!

Peripherials:
    Boot ROM - 2kB eeprom
    MMU
        - 2kB pages
        - TLB with 2(?) hard coded (boot, MMU control, ...) and 8 dynamic records
        - software page fault handling
        - max 32MB of addressable physical memory. That leaves 16MB for RAM and another 16MB for memory mapped peripherial control.
    RAM
    GPU
        - 256x256, 2 bit, 4 item palette of 3:3:2 RGB colors
        - double buffered
        - VGA output?
    Serial interface (USB?) + 2 digit 7segment display
    Storage - SD card?
    Network - PPP or SLIP?
    RTC - interrupts at given rate and at set time

    Raspbery PI as an ultimate peripherial
        - providing everything from the list (perhaps except boot ROM and MMU)
        - would need a memory bus => SPI bridge
            - 25bit address + 2bit flags => 4B address + 1B data => 5B * 4Mhz CPU clock speed = 160Mbit/s
            - too fast, Raspi can only do 125Mhz, Atmega8 f_osc/2

Modes:
    run - normal mode, clock is running
        - 1st tick fetch and decode
        - 2nd tick execute
    sleep - clock is stopped
        - Can only be exited via interrupt

Instruction parameter types:
    i - Value with no interpretation, copied bit by bit.
    u - Unsigned offset.
    j - Signed jump offset, assembler accepts a label as a value for this.

Calling convention:
    None of the registers are preserved through a call
    A register during call for call and ret, its value is clobbered.
    B register used to pass first pointer argument (in left to right order)
    Remaining arguments pushed on stack in left to right order when not variadic, right to left when variadic
    Calee cleans the stack
    if return value is a pointer, it is returned in B, otherwise on stack

Memory model:
    Separate data and program segments
    Virtual address format: PPP PPPS AAAA | AAAA AAAA AAAA
        S - segment (0 = data segment, 1 = program segment)
        P - 6bit process ID (Zero when accessing program segment during interrupt)
        A - 16bit address
    MMU:
        Built out of two 2k * 8b SRAM ICs
        Record format: RWFF FFFF FFFF FFFF
            R - Read allowed
            W - Write allowed
            F - Frame address (14b)
        Hardcoded pages:
            Page 000 0000 0000 (PID = 0, S = 0, A = 0x0000 - 0x0FFF) -> MMU SRAM itself
                - Use as stack space during early bootup
            Page 000 0001 0000 -> 1000 0000 0000 0000 (boot EEPROM page 0, read only)
                -  Need to map further pages manually

            -> Unused physical addresses in the MMU (Records covered by the hardcoded pages):
                000 0000 0000 | 0000 0000 0000 (0x0000)
                000 0000 0000 | 0000 0000 0001 (0x0001)
                000 0000 0000 | 0000 0010 0000 (0x0020)
                000 0000 0000 | 0000 0010 0001 (0x0021)
                Use these to signal failed lookups? Would need an external registers, though.
    12b page size -> 4kB pages
    26b physical address -> 64MB physical address space

Microcode:
    Input:
        8b instruction
        3b uPC
        1b interrupt flag
        1b zero flag
        1b carry
        (14b ~> 16k)
    Output:
        1b ResetUPc

"""

# Instructions:

# Funky multi-cycle instructions wishlist:
#
#   shl, shr

def _predec_mem_read(reg):
    return [f"Read{reg}Addr", "AddrOpMinusOne", f"Load{reg}Addr", "MemRead"] # TODO how is the PRE part done?
def _postinc_mem_read(reg):
    return [f"Read{reg}Addr", "AddrOpOne", f"Load{reg}Addr", "MemRead"] # TODO: how is the POST  part done?
def _predec_mem_write(reg):
    return [f"Read{reg}Addr", "AddrOpMinusOne", f"Load{reg}Addr", "MemWrite"]
_pc_read = _postinc_mem_read("Pc") + ["MemP"]
_stack_pop = _postinc_mem_read("Sp")
_stack_push = _predec_mem_write("Sp")
_end_instruction = Cond(interrupt == 0,
                        _pc_read + ["LoadI", "ResetUPc"],
                        ["PcFromInt", "ReadPcAddr", "AddrOpOne", "LoadIPcAddr"]) # TODO: Just incremented IPc

instructions = {
    "nop": (
        "No operation",
        [
            _end_instruction
        ]
    ),

    ###########################################################################
    # Forth-like stack operations
    ###########################################################################
    "swap": (
        "Swap two topmost items on stack",
        [
            ["ReadTos1", "AluDataBus", "Tos1FromTos0", "SetTos0", "SetTos1"],
            _end_instruction
        ]
    ),
    "dup": (
        "Duplicate top of stack value",
        [
            _stack_push + ["ReadTos1", "Tos1FromTos0", "SetTos1"],
            _end_instruction
        ]
    ),
    "drop": (
        "Remove the topmost item from stack",
        [
            ["ReadTos1", "LoadTos0", "AluDataBus"],
            _stack_pop + ["LoadTos1"],
            _end_instruction
        ]
    ),
    "nip": (
        "Drop second to top of stack",
        [
            _stack_pop + ["LoadTos1"],
            _end_instruction
        ]
    ),
    "tuck": (
        "Copy the top stack item below the second item",
        [
            _stack_push + ["ReadTos0"],
            _end_instruction
        ]
    ),
    "over": (
        "[--Sp] = Tos1; Tos0, Tos1 = Tos1, Tos0 (Copy the second item to top)",
        [
            _stack_push + ["ReadTos1", "LoadTos0", "AluDataBus", "Tos1FromTos0"],
            _end_instruction
        ]
    ),

    ###########################################################################
    # Arithmetic/logic operations
    ###########################################################################
    "bit_not": (
        "Tos0 = ~Tos0",
        [
            ["LoadTos0", "AluNot"],
            _end_instruction
        ]
    ),
    "negate": (
        "Tos0 = -Tos0",
        [
            ["LoadTos0", "AluNeg"],
            _end_instruction
        ]
    ),
    "bit_shift_right": (
        "Tos0 = Tos0 >> 1",
        [
            ["LoadTos0", "AluShr"],
            _end_instruction
        ]
    ),
    # "bit_shift_right_sign":     ("000_ ____", "Tos0 = Tos0 >> 1 | Tos0 & 0x80"),
    # "bit_shift_right_carry":    ("000_ ____", "Tos0 = Tos0 >> 1 | C << 7"),
    # "bit_shift_left":           ("000_ ____", "Tos0 = Tos0 << 1"),
    # "bit_shift_left_circular":  ("000_ ____", "Tos0 = Tos0 << | ((Tos0 & 0x80) >> 7)"),
    # "bit_shift_left_carry":     ("000_ ____", "Tos0 = Tos0 << 1 | C"),
    # "is_negative":          ("000_ ____", "Tos0 = Tos0 & 0x80"), # TODO: Does this actually work correctly?
    # "inc":                  ("000_ ____", "Tos0 += 1"),
    # "inc_carry":            ("000_ ____", "Tos0 += C"),
    # "dec_carry":            ("000_ ____", "Tos0 += 0xff"), #TODO: DECC and INCC are the same?
    # "add":                  ("0100 ___0", "Tos0 = Tos1 + Tos0; Tos1 = [Sp++]"),
    # "add_carry":            ("0100 ___1", "Tos0 = Tos1 + Tos0 + C; Tos1 = [Sp++]"),
    # "sub":                  ("0100 ___0", "Tos0 = Tos1 + ~Tos0 + 1; Tos1 = [Sp++]"),
    # "sub_carry":            ("0100 ___1", "Tos0 = Tos1 + ~Tos0 + C; Tos1 = [Sp++]"), # TODO: Check carry
    # "bit_and":              ("0100 ____", "Tos0 = Tos1 & Tos0; Tos1 = [Sp++]"),
    # "bit_or":               ("0100 ____", "Tos0 = Tos1 | Tos0; Tos1 = [Sp++]"),
    # "bit_xor":              ("0100 ____", "Tos0 = Tos1 ^ Tos0; Tos1 = [Sp++]"),
    # "compare_lt"            ("0100 ____", "Tos0 = Tos1 < Tos0; Tos1 = [Sp++]"),
    # "compare_le"            ("0100 ____", "Tos0 = Tos1 <= Tos0; Tos1 = [Sp++]"),
    "sign_extend": (
        "Push a byte onto stack with top bit of Tos0 repeated",
        [
            _stack_push + ["Tos1FromTos0", "AluSex"],
            _end_instruction
        ]
    ),

    ###########################################################################
    # Loads / stores
    ###########################################################################
    "push_immediate": (
        "Push value from the following program byte",
        [
            _stack_push + ["ReadTos1"],
            ["ReadTos0", "LoadTos1"],
            _pc_read + ["LoadTos0"],
            _end_instruction
        ],
    ),
    # "load_a":               ("0010 0000", "Tos0 = [A]"),
    # "load_a_inc":           ("0010 0001", "Tos0 = [A++]"),
    # "load_a_dec":           ("0010 0010", "Tos0 = [--A]"),
    # "load_b":               ("0010 0100", "Tos0 = [B]"),
    # "load_b_inc":           ("0010 0110", "Tos0 = [B++]"),
    # "load_b_dec":           ("0010 0110", "Tos0 = [--B]"),
    # "store_a":              ("0010 1000", "[A] = Tos0"),
    # "store_a_inc":          ("0010 1001", "[A++] = Tos0"),
    # "store_a_dec":          ("0010 1010", "[--A] = Tos0"),
    # "store_b":              ("0010 1100", "[B] = Tos0"),
    # "store_b_inc":          ("0010 1101", "[B++] = Tos0"),
    # "store_b_dec":          ("0010 1010", "[--B] = Tos0"),

    # TODO: Add relative addressing modes?
    # "load_a_relative"
    # "load_b_relative"
    # "load_stack_relative"

    ###########################################################################
    # Jumps
    ###########################################################################
    "call_a": (
        "Jump to an address stored in A and store the return address in A.",
        [
            _stack_push + ["ReadTos1"],
            _stack_push + ["ReadTos0"],
            ["ReadPcAddr", "LoadTosPair"],
            ["ReadAAddr", "LoadPcAddr"],
            _end_instruction,
        ]
    ),
    "call_immediate": (
        "Jump to an immediate 16 bit value, store return address in A.",
        [
            _pc_read + ["LoadTmp"],
            ["ReadTmp", "DtoAL", "LoadAL"],
            _pc_read + ["LoadTmp"],
            ["ReadTmp", "DtoAH", "LoadAH"],
            _end_instruction
        ]
    ),
    "rjump": (
        "Jump by a immediate signed 8bit distance. 3 cycles.",
        [
            _pc_read + ["LoadTmp"],
            ["ReadPcAddr", "AddrOpTmp", "LoadPcAddr"],
            _end_instruction,
        ]
    ),
    "rjump_if": (
        "Pop a value from the top of stack and jump by a signed 8bit distance if the value was nonzero. 4 cycles.",
        [
            _pc_read + ["LoadTmp"],
            Cond(Tos0 != 0, ["ReadPcAddr", "AddrOpTmp", "LoadPcAddr", "LoadTos0", "ReadTos1"], ["LoadTos0", "ReadTos1"]),
            _stack_pop + ["LoadTos1"],
            _end_instruction,
        ]
    ),

    ###########################################################################
    # Value instructions
    ###########################################################################

    # "swap_a_b":               ("000_ ____", "B, A = A, B"),
    # "swap_a_int":             ("000_ ____", "Int, A = A, Int"),
    # "swap_a_sp":              ("000_ ____", "Sp, A = A, Sp"),
    # "swap_a_pc":              ("000_ ____", "Pc, A = A, Pc"),
    "set_a_low":            ("0100 ____", "A.lo8 = Tos0; Tos0 = Tos1, Tos1 = [Sp++]"), # TODO: Maybe don't pop from the stack, just use TOS?
    "set_a_high":           ("0100 ____", "A.hi8 = Tos0; Tos0 = Tos1, Tos1 = [Sp++]"), # - " -
    "set_b_high":           ("0100 ____", "B.hi8 = Tos0; Tos0 = Tos1, Tos1 = [Sp++]"), # - " -
    "set_b_low":            ("0100 ____", "B.lo8 = Tos0; Tos0 = Tos1, Tos1 = [Sp++]"), # - " -

    "push_a_low": (
        "0110 0___",
        "[--Sp] = Tos1; Tos1 = Tos0; Tos0 = A.lo8",
        [
            _stack_push +
        ]
    ),
    "push_a_high":          ("0110 0___", "[--Sp] = Tos0; Tos0 = A.hi8"),
    "push_b_low":           ("0110 0___", "[--Sp] = Tos0; Tos0 = B.lo8"),
    "push_b_high":          ("0110 0___", "[--Sp] = Tos0; Tos0 = B.hi8"),
    "push_bit":             ("0110 1uuu", "[--Sp] = Tos0, Tos0 = 1 << u"), # TODO: Is it worth having extra 1-of-8 decoder for this?
    "push":                 ("0111 iiii", "[--Sp] = Tos0, Tos0 = value"),

    ###########################################################################
    # Management instructions
    ###########################################################################
    # "reset":                ("000_ ____", "Reset the internal state of the CPU (set all registers to 0)."),
    # "sleep":                ("000_ ____", "Switch into sleep mode."),
    # "interrupt":            ("000_ ____", "Raise interrupt 15."),

    ###########################################################################
    # Complex instructions
    ###########################################################################
    "memcpy": (
        "____ ____",
        """Copy Tos0 bytes pointed to by A to a location pointed to by B.
        Clobbers Tos1. After the operation Tos0 is 0, A and B point one byte
        after the valid ranges, Tos1 is the last byte copied.
        2 cycles / byte """,
        [
            Cond(Tos0 != 0, _postinc_mem_read("A") + ["LoadTos1", "AluDec"], _end_instruction),
            _postinc_mem_write("B") + ["ReadTos1", "ResetUPc"],
        ]
    ),

    "strncpy": (
        """Copy at most Tos0 bytes pointed to by A to a location pointed to by B,
        terminating on zero byte. After the operation Tos0 is 0 or the number
        of bytes from the end where zero byte was encountered, A and B point
        one byte after the last copied byte, Tos1 is the last byte copied.
        3 cycles / byte""",
        [
            Cond(
                Tos0 != 0,
                _postinc_mem_read("A") + ["LoadTos1", "AluDec"],
                _end_instruction
            ),
            _postinc_mem_write("B") + ["ReadTos0", "Tos1FromTos0"],
            Cond(
                Tos0 != 0,
                ["ReadTos1", "LoadTos0", "Tos1FromTos0", "ResetUPc"],
                ["ReadTos1", "LoadTos0", "Tos1FromTos0"]
            ),
            _end_instruction
        ],
    ),
    "multiply_accumulate": (
        "Calculate A += Tos0 * B (16bit + 8bit * 16bit). Afterwards Tos0 is zero, . 1 - 17 cycles",
        [
            Cond(
                Tos0 != 0,
                ["AluShr", "ReadBAddr", "AddrOpB", "LoadBAddr"], # shift right Tos0, store bit in C, double B
                _end_instruction
            ),
            Cond(
                C != 0,
                ["ReadAAddr", "AddrOpB", "LoadAAddr", "ResetUPc"], # Add B into A
                ["ResetUPc"]
            )
        ]
    )
}

if __name__ == "__main__":
    import pprint
    print("Instructions:")
    pprint.pprint(instructions)

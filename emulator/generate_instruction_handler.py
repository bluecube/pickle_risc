#!/usr/bin/env python3

import pyjson5 as json5

import argparse

instruction_bits = 16
opcode_bits = 7

parser = argparse.ArgumentParser(description='Generate C instruction definitions')
parser.add_argument('input', metavar='INPUT', type=argparse.FileType("r"),
                    help='Instruction definition file')
parser.add_argument('--output', "-o", type=argparse.FileType("w"), default=None,
                    help='C file where to store the generated code')
parser.add_argument('--header-name', type=str, default=None,
                    help='name of the header file to use for #include.')
args = parser.parse_args()

data = json5.load(args.input)
outfile = args.output
header_name = args.header_name

outfile.write(f"#include \"{header_name}\"\n")
outfile.write("#include <stdbool.h>\n")
outfile.write("#include <stdio.h>\n")
outfile.write("\n")


def concretize_encoding(enc):
    """
    Returns a generator of concrete encodings, replacing "-" characters with
    all combinations of 0 and 1.
    """

    first_dash = enc.find("-")
    if first_dash == -1:
        yield enc
        return

    prefix0 = enc[:first_dash] + "0"
    prefix1 = enc[:first_dash] + "1"
    for rest in concretize_encoding(enc[first_dash + 1:]):
        assert len(rest) == len(enc) - len(prefix0)
        yield prefix0 + rest
        yield prefix1 + rest


def handler_name(mnemonic):
    if mnemonic is None:
        return "invalid_instruction"
    else:
        return f"handle_{mnemonic}"


def handler_encodings(mnemonic, details):
    encoding = ""
    for encoding_piece in details["encoding"]:
        if encoding_piece in details.get("args", {}):
            arg_type = details["args"][encoding_piece]
            if arg_type in ["gpr", "cr"]:
                arg_size = 3
            else:
                arg_size = int(arg_type[1:])

            encoding += "-" * arg_size  # Using "-" as don't care flag
        elif all(c in "01x" for c in encoding_piece):
            encoding += encoding_piece.replace("x", "-")
        else:
            raise ValueError(f"Unknown encoding piece {encoding_piece} for instruction {mnemonic}")

    if len(encoding) != instruction_bits:
        raise ValueError(f"Encoding for instruction {mnemonic} has length {len(encoding)}")

    if any(c != "-" for c in encoding[opcode_bits:]):
        raise ValueError(f"Encoding for instruction {mnemonic} has required bits outside of opcode")

    yield from concretize_encoding(encoding[:opcode_bits])


def instruction_handler(mnemonic, details, outfile):
    outfile.write(f"// Handler for instruction {mnemonic!r}\n")
    outfile.write(f"// ({details['title']})\n")
    outfile.write(f"static bool {handler_name(mnemonic)}(struct cpu_state *state) {{\n")
    outfile.write(f'    (void)state;\n')
    outfile.write(f'    printf("{mnemonic}\\n");\n')
    outfile.write(f'    return true;\n')
    outfile.write("}\n")
    outfile.write("\n")


opcode_table = [None] * (1 << opcode_bits)
for mnemonic, details in data["instructions"].items():
    for enc in handler_encodings(mnemonic, details):
        opcode_table[int(enc, base=2)] = mnemonic

    instruction_handler(mnemonic, details, outfile)

instruction_handler(None, data["invalid_instruction"], outfile)


outfile.write("bool handle_instruction(word_t instruction, struct cpu_state *state) {\n")
shift_distance = instruction_bits - opcode_bits
outfile.write(f"    switch (instruction >> {shift_distance}) {{\n")
for i, mnemonic in enumerate(opcode_table):
    outfile.write(f"        case 0x{i:02x}: return {handler_name(mnemonic)}(state);\n")
outfile.write(f"        default: __builtin_unreachable();\n")
outfile.write("    };\n")
outfile.write("};\n")

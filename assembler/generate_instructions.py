#!/usr/bin/env python3

import pyjson5 as json5

import argparse
import itertools
import copy

instruction_bits = 16
max_args = 3

def substituted_instructions(instructions, substitutions):
    for mnemonic, details in instructions.items():
        used_substitutions = [n for n in substitutions.keys() if ("{" + n + "}") in mnemonic]

        if not used_substitutions:
            yield mnemonic, details, lambda x, y: x
        else:
            for values in itertools.product(*[substitutions[n].items() for n in used_substitutions]):
                mnemonic_copy = copy.deepcopy(mnemonic)

                for n, (mnemonic_v, details_v) in zip(used_substitutions, values):
                    mnemonic_copy = mnemonic_copy.replace("{" + n + "}", mnemonic_v)

                def subst_function(string, field):
                    for n, (mnemonic_v, details_v) in zip(used_substitutions, values):
                        if field in details_v:
                            string = string.replace("{" + n + "}", details_v[field])
                    return string

                yield mnemonic_copy, details, subst_function


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
outfile.write("#include <stddef.h>\n")
outfile.write("struct instruction instructions[] = {\n")

for mnemonic, details, subst_function in substituted_instructions(data["instructions"], data["substitutions"]):
    encoding = ""
    arg_locations = {}
    for encoding_piece in details["encoding"]:
        encoding_piece = subst_function(encoding_piece, "encoding")
        if encoding_piece in details.get("args", {}):
            arg_type = details["args"][encoding_piece]
            if arg_type in ["gpr", "cr"]:
                arg_size = 3
            else:
                arg_size = int(arg_type[1:])

            encoding += "0" * arg_size
            arg_locations[encoding_piece] = (len(encoding), arg_size)
        elif all(c in "01x" for c in encoding_piece):
            encoding += encoding_piece.replace("x", "0")
        else:
            raise ValueError(f"Unknown encoding piece {encoding_piece} for instruction {mnemonic}")

    if len(encoding) != instruction_bits:
        raise ValueError(f"Encoding for instruction {mnemonic} has length {len(encoding)}")

    outfile.write("    {\n")
    outfile.write(f"        .mnemonic = \"{mnemonic}\",\n")
    outfile.write(f"        .encoding = 0x{int(encoding, 2):04x},\n")
    outfile.write("        .args = {\n")

    if len(details.get("args", {})) > max_args:
        raise ValueError(f"Instruction {mnemonic} has {len(args)} arguments (max is {max_args}).")
    for arg_name, arg_type in details.get("args", {}).items():
        if arg_type in ["gpr", "cr"]:
            output_arg_type = "INSTRUCTION_ARG_" + arg_type.upper();
        else:
            if arg_type[0] == "s":
                signed = "SIGNED"
            elif arg_type[0] == "u":
                signed = "UNSIGNED"
            else:
                raise ValueError(
                    f"Bad argument type {arg_type} "
                    f"for argument {encoding_piece}, "
                    f"instruction {mnemonic}"
                )
            output_arg_type = "INSTRUCTION_ARG_" + signed

        try:
            arg_pos, arg_size = arg_locations[arg_name]
        except KeyError as e:
            raise ValueError(f"Argument {arg_name} for instruction {mnemonic} mising in encoding") from e

        shift = instruction_bits - arg_pos
        outfile.write(f"            {{ .type = {output_arg_type}, .shift = {shift}, .size = {arg_size} }},\n")

    outfile.write("            { .type = INSTRUCTION_ARG_NONE }\n")
    outfile.write("        }\n")
    outfile.write("    },\n")

outfile.write("    { .mnemonic = NULL }\n")
outfile.write("};\n")

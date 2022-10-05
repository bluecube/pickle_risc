#!/usr/bin/env python3
import pyjson5 as json5
import dominate
from dominate.tags import *
import markdown

import sys
import argparse
import re


def multiline(v):
    """ Process a possibly multiline text from the JSON """
    if isinstance(v, str):
        yield v
    else:
        yield from v


def generate_arg(name):
    return var(name, __inline=True)


def generate_mnemonic(name, link=True):
    if link:
        return a(name, href=f"#instruction-{name}", cls="mnemonic", __inline=True)
    else:
        return span(name, cls="mnemonic", __inline=True)


@table(cls="encoding")
def generate_encoding_table(encoding, args, menmonic):
    with tr():
        for i in reversed(range(16)):
            th(str(i))
    descriptions_row = []  # (text, colspan)
    with tr():
        for piece in encoding:
            if all(x in "01" for x in piece):
                td(code(piece), colspan=len(piece), rowspan=2)
            elif all(x == "x" for x in piece):
                td("ignored", colspan=len(piece), rowspan=2, cls="placeholder")
            elif piece in args:
                arg_type = args[piece]
                if arg_type == "gpr":
                    arg_desc = "register"
                    arg_size = 3
                elif arg_type == "cr":
                    arg_desc = "control register"
                    arg_size = 3
                else:
                    if arg_type[0] == "s":
                        signed = True
                    elif arg_type[0] == "u":
                        signed = False
                    else:
                        raise ValueError(f"Unexpected argument type {arg_type!r}, ({mnemonic}/{piece})")

                    arg_size = int(arg_type[1:])
                    arg_desc = f"{arg_size}b {'signed' if signed else 'unsigned'} integer"

                td(generate_arg(piece), colspan=arg_size)
                descriptions_row.append((arg_desc, arg_size))
            else:
                raise ValueError(f"Unexpected encoding piece {piece!r} ({mnemonic})")
    with tr():
        for arg_desc, arg_size in descriptions_row:
            td(arg_desc, colspan=arg_size)


@p(cls="asm_syntax")
def generate_syntax(mnemonic):
    asm_syntax_element = code(cls="asm_syntax")
    asm_syntax_element += generate_mnemonic(mnemonic, link=False)
    first_arg = True
    for arg in instruction_args.keys():
        if first_arg:
            asm_syntax_element += " "
            first_arg = False
        else:
            asm_syntax_element += ", "
        asm_syntax_element += generate_arg(arg)


@pre
def generate_highlighted_pseudocode(pseudocode, args, all_instructions):
    pseudocode_element = code(cls="pseudocode")
    first_row = True
    for line in multiline(pseudocode):
        pos = 0
        if first_row:
            first_row = False
        else:
            pseudocode_element += "\n"

        for match in re.finditer(r"[a-zA-Z_0-9]+|{[a-zA-Z_0-9]+}|#.*$", line):
            pseudocode_element += line[pos:match.start()]
            pos = match.end()
            matched = match[0]

            if matched in args:
                pseudocode_element += generate_arg(matched)
            elif matched in all_instructions:
                pseudocode_element += generate_mnemonic(matched)
            elif matched[0] == "#":
                pseudocode_element += span(matched, cls="comment")
            elif matched in ["if", "else", "apply", "while"]:
                pseudocode_element += span(matched, cls="keyword")
            else:
                # Default without any highlight
                pseudocode_element += matched
        pseudocode_element += line[pos:]


def generate_optional_markdown_block(identifier, details, title):
    if identifier not in details:
        return None

    with div(cls=identifier, __inline=False) as d:
        if title is not None:
            h4(title)
        raw_html = markdown.markdown("\n".join(multiline(details[identifier])))
        dominate.util.raw(raw_html)

    return d


parser = argparse.ArgumentParser(description='Generate HTML instruction set documentation')
parser.add_argument('input', metavar='INPUT', type=argparse.FileType("r"),
                    help='Instruction definition file')
parser.add_argument('--output', "-o", type=argparse.FileType("w"), default=sys.stdout,
                    help='HTML file where to store the generated code')
args = parser.parse_args()

data = json5.load(args.input)

title = "Pickle RISC instruction set"

document = dominate.document(title=title)
with document.head:
    style(r"""
        pre code {
            display: block;
            width: 50em;
        }

        code, var {
            background-color: #eee;
            font-family: monospace;
            font-style: normal;
        }

        var {
            font-style: italic;
        }

        code .mnemonic {
            font-weight: bold;
            color: #00a;
        }

        code .comment {
            color: #060;
        }

        code .keyword {
            color: #700;
        }

        code .comment {
            color: #080;
        }

        .placeholder {
            color: #999;
        }

        .placeholder::before {
            content: '\3C';
        }

        .placeholder::after {
            content: '\3E';
        }

        table {
            border-collapse: collapse;
        }

        table td {
            padding-left: 1em;
            padding-right: 1em;
        }


        table.encoding {
            table-layout: fixed;
        }

        table.encoding td {
            border: .01em solid black;
            text-align: center;
            padding: 0.2em;
        }

        table.encoding th {
            font-weight: normal;
            width: 2em;
        }

        ul.asm_syntax {
            list-style: none;
            padding: 0;
        }

    """, type="text/css")
with document:
    h1(title)

    with section():
        h2("Instructions")
        for mnemonic, details in data["instructions"].items():
            with section(id="instruction-" + mnemonic):
                instruction_args = details.get("args", {})

                h3(re.sub(r"{[a-z_]+}", "", details["title"]).strip())

                generate_optional_markdown_block("description", details, None)

                h4("Encoding")
                generate_encoding_table(details["encoding"], instruction_args, mnemonic)

                h4("Syntax")
                generate_syntax(mnemonic)

                if "pseudocode" in details:
                    h4("Pseudocode")
                    generate_highlighted_pseudocode(
                        details["pseudocode"],
                        instruction_args,
                        data["instructions"]
                    )

                if "note" in details:
                    for note in multiline(details["note"]):
                        with div(cls="note", __inline=False):
                            h4("Note")
                            raw_html = markdown.markdown(note)
                            dominate.util.raw(raw_html)

args.output.write(document.render())

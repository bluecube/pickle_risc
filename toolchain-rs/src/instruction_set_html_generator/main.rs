use clap::Parser;
use clio::{Input, Output};
use lazy_static::lazy_static;
use maud::{html, Markup, DOCTYPE};
use regex::Regex;

use instruction_set::{
    Instruction, InstructionEncodingArgType, InstructionEncodingPiece, InstructionSet,
    INSTRUCTION_BITS,
};

use std::io::Write;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to instruction set definition file (json5)
    #[arg(default_value = "-")]
    input: Input,

    /// Path to store the HTML output
    #[arg(short, long, default_value = "-")]
    output: Output,
}

fn main() -> anyhow::Result<()> {
    let mut cli = Cli::parse();
    let definition = InstructionSet::read(cli.input)?;

    write!(cli.output, "{}", render_html(&definition).into_string())?;

    Ok(())
}

fn render_html(instruction_set: &InstructionSet) -> Markup {
    let title = "Pickle RISC instruction set";
    html! {
        (DOCTYPE)
        head {
            title { (title) }
            style type="text/css" { (CSS) }
        }
        body {
            h1 { (title) }

            section {
                h2 { "Overview" }
                (render_instruction_set_overview(instruction_set))
            }

            section {
                h2 { "Instructions" }
                (render_instructions(instruction_set))
            }
        }
    }
}

fn render_instruction_set_overview(instruction_set: &InstructionSet) -> Markup {
    html! {
        table .encoding {
            colgroup {
                col;
                col span=(INSTRUCTION_BITS);
            }
            thead {
                tr {
                    @for i in 0..INSTRUCTION_BITS {
                        th { ((INSTRUCTION_BITS - i - 1)) }
                    }
                    th .title {  }
                }
            }
            tbody {
                @for (mnemonic, instr) in instruction_set.instructions.iter() {
                    tr {
                        @for piece in instr.encoding_pieces.iter() {
                            (render_instruction_encoding_piece_td(&piece, instr, true))
                        }
                        th .title { a href={"#instruction-" (mnemonic)} { (instr.title) } }
                    }
                }
            }
        }
    }
}

fn render_instructions(instruction_set: &InstructionSet) -> Markup {
    html! {
        @for (mnemonic, instr) in instruction_set.instructions.iter() {
            (render_instruction(mnemonic, instr, instruction_set))
        }
    }
}

fn render_instruction(
    mnemonic: &str,
    instruction: &Instruction,
    instruction_set: &InstructionSet,
) -> Markup {
    html! {
        section #{"instruction-" (mnemonic)} {
            h3 { (mnemonic) ": " (instruction.title) }
            h4 { "Encoding" }
            (render_single_instruction_encoding(instruction))
            h4 { "Assembler syntax" }
            p { (render_asm(mnemonic, instruction)) }
            @if let Some(pseudocode) = &instruction.pseudocode {
                h4 { "Pseudocode" }
                p { (render_pseudocode(pseudocode, instruction, instruction_set)) }
            }
        }
    }
}

fn render_single_instruction_encoding(instruction: &Instruction) -> Markup {
    html! {
        table .encoding {
            thead {
                tr {
                    @for i in 0..INSTRUCTION_BITS {
                        th { ((INSTRUCTION_BITS - i - 1)) }
                    }
                }
            }
            tbody {
                tr {
                    @for piece in instruction.encoding_pieces.iter() {
                        (render_instruction_encoding_piece_td(&piece, instruction, false))
                    }
                }
            }
        }
    }
}

fn render_instruction_encoding_piece_td(
    piece: &InstructionEncodingPiece,
    instruction: &Instruction,
    literal_colspans: bool
) -> Markup {
    match piece {
        InstructionEncodingPiece::Literal(s) => html! {
            @if literal_colspans {
                td colspan=(s.len()) {
                    code { (s) }
                }
            } @else {
                @for b in s.chars() {
                    td { code { (b) } }
                }
            }
        },
        InstructionEncodingPiece::Ignored(bits) => html! {
            td .placeholder colspan=(bits) {}
        },
        InstructionEncodingPiece::Arg(name) => {
            let arg_type = instruction.args.get(name).unwrap();
            html! {
                td colspan=(arg_type.bits()) {
                    var { (name) } br; (render_arg_type(arg_type))
                }
            }
        }
    }
}

fn render_arg_type(arg_type: &InstructionEncodingArgType) -> Markup {
    match arg_type {
        InstructionEncodingArgType::Gpr => html! { "register" },
        InstructionEncodingArgType::ControlRegister => html! { "control reg" },
        InstructionEncodingArgType::Immediate { signed, bits } => html! {
            (bits) "b " @if !signed { "un" } "signed int"
        },
    }
}

fn render_asm(mnemonic: &str, instruction: &Instruction) -> Markup {
    html! {
        code .asm {
            span .mnemonic { (mnemonic) }
            " "
            @for (i, (arg, _)) in instruction.args.iter().enumerate() {
                @if i > 0 {
                    ", "
                }
                var { (arg) }
            }
        }
    }
}

fn render_pseudocode<P: Clone + Into<Vec<String>>>(
    pseudocode: &P,
    instruction: &Instruction,
    instruction_set: &InstructionSet,
) -> Markup {
    html! {
        code .pseudocode {
            @for (i, line) in pseudocode.clone().into().into_iter().enumerate() {
                @if i > 0 { "\n" }
                (render_pseudocode_line(&line, instruction, instruction_set))
            }
        }
    }
}

fn render_pseudocode_line(
    line: &str,
    instruction: &Instruction,
    instruction_set: &InstructionSet,
) -> Markup {
    lazy_static! {
        static ref HIGHLIGHT_RE: Regex = Regex::new(r"[a-zA-Z_0-9]+|[0-9]+|//.*$|.+?").unwrap();
    }
    html! {
        @for matched in HIGHLIGHT_RE.find_iter(line) {
            @let matched = matched.as_str();
            @if matched.starts_with("//") {
                span .comment { (matched) }
            } @else if matched.chars().next().unwrap().is_digit(10) {
                span .number { (matched) }
            } @else if matched == "if" || matched == "else" || matched == "for" || matched == "while" {
                span .keyword { (matched) }
            } @else if instruction.args.get(matched).is_some() {
                var { (matched) }
            } @else if instruction_set.instructions.get(matched).is_some() {
                span .mnemonic { (matched) }
            } @else {
                (matched)
            }
        }
    }
}

const CSS: &'static str = r#"
code.pseudocode, code.asm {
    display: block;
    white-space: pre-wrap;
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

code .number {
    color: #559;
}

.placeholder {
    color: #999;
}

.placeholder::before {
    content: '\3Cignored\3E';
}

table {
    border-collapse: collapse;
    margin-left: auto;
    margin-right: auto;
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

table.encoding th.title {
    font-weight: normal;
    width: auto;
    padding-left: 1em;
    padding-right: 1em;
    text-align: left;
}

table.encoding code {
    font-size: 125%;
}

"#;

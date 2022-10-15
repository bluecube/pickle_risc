use std::env;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::collections::BTreeMap;
use std::iter::repeat;
use std::ops::Range;

use serde::Deserialize;
use itertools::Itertools;
use anyhow;
use thiserror::Error;

#[derive(Deserialize, Debug)]
struct InstructionSet {
    instructions: BTreeMap<String, InstructionDef>,
    invalid_instruction_microcode: Option<Vec<Vec<String>>>
}

#[derive(Deserialize, Debug)]
struct InstructionDef {
    #[serde(default)]
    args: BTreeMap<String, InstructionEncodingArgType>,
    encoding: Vec<InstructionEncodingPiece>,
    microcode: Option<Vec<Vec<String>>>
}

#[derive(Deserialize, Debug, Copy, Clone)]
#[serde(try_from = "String")]
enum InstructionEncodingArgType {
    Gpr,
    ControlRegister,
    Immediate { signed: bool, bits: usize }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(from = "String")]
enum InstructionEncodingPiece {
    Literal(String),
    Ignored(usize),
    Arg(String)
}

#[derive(Error, Debug)]
#[error("{0} is not a valid instruction argument type (should match `gpr|cr|[su][0-9]+`)")]
struct ParseInstructionEncodingArgTypeError(String);

#[derive(Error, Debug)]
enum InstructionDefinitionError {
    #[error("{arg_name:?} is not an argument of instruction {mnemonic}")]
    UndefinedArgument { mnemonic: String, arg_name: String },

    #[error("Instruction {mnemonic} has bad encoding length")]
    WrongEncodingLength { mnemonic: String, bits: usize },

    #[error("Bad microcode for instruction {mnemonic}: {details}")]
    MicrocodeError { mnemonic: String, details: String }
}

impl TryFrom<&str> for InstructionEncodingArgType {
    type Error = ParseInstructionEncodingArgTypeError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "gpr" => Ok(InstructionEncodingArgType::Gpr),
            "cr" => Ok(InstructionEncodingArgType::ControlRegister),
            _ => {
                let signed = match &s[0..1] {
                    "s" => true,
                    "u" => false,
                    _ => return Err(ParseInstructionEncodingArgTypeError(s.into()))
                };
                let bits = s[1..].parse::<usize>().map_err(|_| ParseInstructionEncodingArgTypeError(s.into()))?;

                Ok(InstructionEncodingArgType::Immediate{signed, bits})
            }
        }
    }
}

impl TryFrom<String> for InstructionEncodingArgType {
    type Error = ParseInstructionEncodingArgTypeError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::try_from(&s[..])
    }
}


impl InstructionEncodingArgType {
    fn bits(&self) -> usize {
        match self {
            InstructionEncodingArgType::Gpr => 3,
            InstructionEncodingArgType::ControlRegister => 3,
            InstructionEncodingArgType::Immediate { signed: _, bits } => *bits
        }
    }
}

impl From<String> for InstructionEncodingPiece {
    fn from(s: String) -> InstructionEncodingPiece {
        if s.chars().all(|c| c == '0' || c == '1') {
            InstructionEncodingPiece::Literal(s)
        } else if s.chars().all(|c| c == 'x') {
            InstructionEncodingPiece::Ignored(s.len())
        } else {
            InstructionEncodingPiece::Arg(s)
        }
    }
}

impl InstructionDef {
    fn encoding(&self, mnemonic: &str, instruction_bits: usize) -> Result<String, InstructionDefinitionError> {
        let mut encoding = String::new();

        for encoding_piece in &self.encoding {
            match encoding_piece {
                InstructionEncodingPiece::Literal(bits) => encoding.push_str(&bits),
                InstructionEncodingPiece::Ignored(count) => for _ in 0..*count { encoding.push('x') },
                InstructionEncodingPiece::Arg(arg_name) => {
                    let arg_size = self.args.get(arg_name)
                        .ok_or_else(
                            || InstructionDefinitionError::UndefinedArgument{
                                mnemonic: mnemonic.into(),
                                arg_name: arg_name.clone()
                            })?
                        .bits();
                    for _ in 0..arg_size { encoding.push('x') }
                }
            }
        }

        if encoding.len() != instruction_bits {
            Err(InstructionDefinitionError::WrongEncodingLength {
                mnemonic: mnemonic.into(),
                bits: encoding.len()
            })
        } else {
            Ok(encoding)
        }
    }
}

fn main() {
    generate_instruction_handler().unwrap();
}

fn generate_instruction_handler() -> anyhow::Result<()> {
    let instruction_bits = 16;
    let opcode_bits = 7;

    let out_dir = env::var_os("OUT_DIR").unwrap();
    let target_path = Path::new(&out_dir).join("instruction_handler.rs");
    let definition_path = Path::new("..").join("instruction_set.json5");

    println!("cargo:warning=Output goes to {}", target_path.to_str().unwrap());
    println!("cargo:rerun-if-changed={}", definition_path.to_str().unwrap());

    let definition_str = fs::read_to_string(definition_path)?;
    let definition = json5::from_str::<InstructionSet>(&definition_str).unwrap();

    let mut target = File::create(target_path)?;

    writeln!(target, "#[allow(unreachable_code)]")?;
    writeln!(target, "match opcode >> {} {{", instruction_bits - opcode_bits)?;
    let opcode_table = make_opcode_table(&definition, opcode_bits, instruction_bits)?;
    for (count, (first_opcode, instruction)) in opcode_table
        .iter()
        .enumerate()
        .dedup_by_with_count(|x, y| x.1.map(|x| x.0) == y.1.map(|x| x.0))
    {

        generate_opcode_match_arm(
            instruction.map(|x| x.0),
            first_opcode..(first_opcode + count),
            if let Some((_, instruction_def)) = instruction {
                &instruction_def.microcode
                // Unwrap is ok, because we got the menmonic from the encodings in the first place
            } else {
                &definition.invalid_instruction_microcode
            },
            &mut target
        )?;
    }
    writeln!(target, "    _ => unreachable!(),")?;
    writeln!(target, "}}")?;

    Ok(())
}

fn make_opcode_table(definition: &InstructionSet, opcode_bits: usize, instruction_bits: usize) -> anyhow::Result<Vec<Option<(&str, &InstructionDef)>>> {
    let mut table: Vec<Option<(&str, &InstructionDef)>> = repeat(None).take(1 << opcode_bits).collect();

    for (mnemonic, instruction_def) in &definition.instructions {
        let encoding = instruction_def.encoding(&mnemonic, instruction_bits)?;
        for opcode in expand_encoding(&encoding[..opcode_bits]) {
            table[opcode] = Some((&mnemonic, &instruction_def));
        }
    }

    Ok(table)
}

/// Converts a str with 0, 1 and other into all numbers that match this bit string
fn expand_encoding(s: &str) -> impl Iterator<Item = usize> {
    s.chars()
        .map(|c| match c { '0' => 0..=0, '1' => 1..=1, _ => 0..=1, })
        .multi_cartesian_product()
        .map(|x| x.iter().fold(0, |acc, digit| (acc << 1) + digit))
}

fn generate_opcode_match_arm(
    mnemonic: Option<&str>,
    opcodes: Range<usize>,
    microcode: &Option<Vec<Vec<String>>>,
    target: &mut fs::File
) -> anyhow::Result<()> {
    if opcodes.len() == 1 {
        writeln!(target, "    {:#04x} => {{", opcodes.start)?;
    } else {
        writeln!(target, "    {:#04x}..={:#04x} => {{", opcodes.start, opcodes.end - 1)?;
    }
    writeln!(target, "        // {}", mnemonic.unwrap_or("invalid instruction"))?;
    if let Some(microcode) = microcode {
        for (i, microcode_step) in microcode.iter().enumerate() {
            generate_microcode_step(i, microcode_step, target)?;
        }
    } else {
        writeln!(target, "        todo!(); // Missing microcode!")?;
    }
    writeln!(target, "    }}")?;

    Ok(())
}

fn generate_microcode_step(step: usize, microcode: &Vec<String>, target: &mut fs::File) -> anyhow::Result<()> {
    const INDENT: &str = "        ";

    writeln!(target, "{}{{ // Microcode step {}", INDENT, step)?;
    writeln!(target, "{}    #[allow(unused_mut,unused_variables)] let mut segment = VirtualMemorySegment::DataSegment;", INDENT)?;

    let mut microinstructions = microcode.iter()
        .map(|microinstruction| translate_microinstruction(microinstruction))
        .collect::<Result<Vec<_>, _>>()?;

    microinstructions.sort_by(|(_, priority1), (_, priority2)| priority1.cmp(priority2));

    for (code, _) in microinstructions {
        writeln!(target, "{}    {}", INDENT, code)?;
    }

    writeln!(target, "{}}}", INDENT)?;

    // TODO: Conditionals in microcode
    Ok(())
}

/// Parse a microinstruction, returns rust code to emulate it, and its priority
/// (to produce microinstructions that produce value before the ones that consume them)
fn translate_microinstruction(microinstruction: &str) -> Result<(String, usize), InstructionDefinitionError> {
    let (code, priority) = match microinstruction {
        "pc->left" => ("let left_bus = self.pc;", 0),
        "pc->addr_base" => ("let addr_base_bus = self.pc;", 0),
        "zero->left" => ("let left_bus = 0;", 0),
        "f2->left" => ("let left_bus = self.get_gpr(field!(opcode, 3));", 0),
        "f3->left" => ("let left_bus = self.get_gpr(field!(opcode >> 3, 3));", 0),
        "f4->right" => ("let right_bus = self.get_gpr(field!(opcode >> 6, 3));", 0),
        "f5->right" => ("let right_bus = self.get_gpr(field!(opcode >> 10, 3));", 0),
        "f6->right" => ("let right_bus = self.get_cr(field!(opcode >> 9, 3));", 0),
        "f7->right" => ("let right_bus = sign_extend(field!(opcode >> 3, 8), 8);", 0),

        "right->addr_base" => ("let addr_base_bus = right_bus;", 1),
        "left->mem_data" => ("let mem_data = left_bus;", 1),

        "alu_add->result" => ("let result_bus = left_bus.wrapping_add(right_bus);", 1),
        "alu_and->result" => ("let result_bus = left_bus & right_bus;", 1),
        "alu_or->result" => ("let result_bus = left_bus | right_bus;", 1),
        "alu_xor->result" => ("let result_bus = left_bus ^ right_bus;", 1),
        "alu_sub->result" => ("let result_bus = left_bus.wrapping_sub(right_bus);", 1),
        "alu_upsample->result" => ("let result_bus = (left_bus & 0xff) | (right_bus & 0xff) << 8;", 1),

        "f8->addr_offset" => ("let mem_address = addr_base_bus.wrapping_add(sign_extend((opcode >> 3) & 0x7F, 7));", 2),
        "zero->addr_offset" => ("let mem_address = addr_base_bus;", 2),
        "one->addr_offset" => ("let mem_address = addr_base_bus.wrapping_add(1);", 2),

        "mem_address->pc" => ("self.pc = mem_address;", 3),
        "read_mem_data" => ("let mem_data = self.read_memory_virt(mem_address);", 3),
        "write_mem_data" => ("self.write_memory_virt(mem_address, left_bus);", 3),

        "mem_data->instruction" => ("self.next_opcode = mem_data;", 4),
        "mem_data->result" => ("let result_bus = mem_data;", 4),

        "result->f1" => ("self.set_gpr(field!(opcode & 0x7, 3), result_bus);", 5),
        "result->f6" => ("self.set_cr(field!(opcode >> 9, 3), result_bus);", 5),
        _ => ("todo!();", 9999)
    };
    Ok((format!("{} // {}", code, microinstruction), priority))
}

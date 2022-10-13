use std::env;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::collections::BTreeMap;
use std::iter::repeat;

use serde::Deserialize;
use itertools::Itertools;
use anyhow;
use thiserror::Error;

#[derive(Deserialize, Debug)]
struct InstructionSet {
    instructions: BTreeMap<String, InstructionDef>,
    //invalid_instruction: InstructionDef
}

#[derive(Deserialize, Debug)]
struct InstructionDef {
    #[serde(default)]
    args: BTreeMap<String, InstructionEncodingArgType>,
    encoding: Vec<InstructionEncodingPiece>
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
    WrongEncodingLength { mnemonic: String, bits: usize }
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

    writeln!(target, "match opcode >> {} {{", instruction_bits - opcode_bits)?;
    let opcode_table = make_opcode_table(&definition, opcode_bits, instruction_bits)?;
    for (count, (first_opcode, mnemonic)) in opcode_table.iter().enumerate().dedup_by_with_count(|x, y| x.1 == y.1) {
        writeln!(target, "    {:#04x}{} => {{",
            first_opcode,
            if count > 1 { format!(" ..= {:#04x}", first_opcode + count - 1) } else { "".to_string() }
        )?;
        writeln!(target, "        // {}", mnemonic.unwrap_or("invalid instruction"))?;
        writeln!(target, "    }}")?;
    }
    writeln!(target, "    _ => unreachable!();")?;
    writeln!(target, "}}")?;

    Ok(())
}

fn make_opcode_table(definition: &InstructionSet, opcode_bits: usize, instruction_bits: usize) -> anyhow::Result<Vec<Option<&str>>> {
    let mut table: Vec<Option<&str>> = repeat(None).take(1 << opcode_bits).collect();

    for (mnemonic, instruction_def) in &definition.instructions {
        let encoding = instruction_def.encoding(&mnemonic, instruction_bits)?;
        for opcode in expand_encoding(&encoding[..opcode_bits]) {
            table[opcode] = Some(&mnemonic);
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

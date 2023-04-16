use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::io::{Read, read_to_string};

use thiserror::Error;
use serde::Deserialize;
use indexmap::IndexMap;

pub const INSTRUCTION_BITS: usize = 16;
pub const OPCODE_BITS: usize = 7; // TODO: This could be extracted from the definition

#[derive(Deserialize, Debug)]
pub struct InstructionSet {
    pub instructions: IndexMap<String, Instruction>,
    pub invalid_instruction_microcode: Option<Vec<Vec<String>>>,
    pub substitutions: HashMap<String, Vec<String>>
}

impl InstructionSet {
    pub fn from_json5(json: &str) -> anyhow::Result<Self> {
        json5::from_str::<Self>(&json).map_err(|e| e.into())
    }

    pub fn load<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let string = fs::read_to_string(path)?;
        Self::from_json5(&string)
    }

    pub fn read(reader: impl Read) -> anyhow::Result<Self> {
        let string = read_to_string(reader)?;
        Self::from_json5(&string)
    }
}

#[derive(Deserialize, Debug)]
pub struct Instruction {
    pub title: String,
    #[serde(default)]
    pub args: IndexMap<String, InstructionEncodingArgType>,
    #[serde(rename="encoding")]
    pub encoding_pieces: Vec<InstructionEncodingPiece>,
    pub pseudocode: Option<OneOrMany<String>>,
    pub note: Option<OneOrMany<String>>,
    pub microcode: Option<Vec<Vec<String>>>
}

impl Instruction {
    pub fn encoding(&self, mnemonic: &str) -> Result<String, InstructionDefinitionError> {
        let mut encoding = String::new();

        for encoding_piece in &self.encoding_pieces {
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

        if encoding.len() != INSTRUCTION_BITS {
            Err(InstructionDefinitionError::WrongEncodingLength {
                mnemonic: mnemonic.into(),
                bits: encoding.len()
            })
        } else {
            Ok(encoding)
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(try_from = "String")]
pub enum InstructionEncodingArgType {
    Gpr,
    ControlRegister,
    Immediate { signed: bool, bits: usize }
}

impl TryFrom<&str> for InstructionEncodingArgType {
    type Error = InstructionDefinitionError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "gpr" => Ok(InstructionEncodingArgType::Gpr),
            "cr" => Ok(InstructionEncodingArgType::ControlRegister),
            _ => {
                let signed = match &s[0..1] {
                    "s" => true,
                    "u" => false,
                    _ => return Err(InstructionDefinitionError::BadArgumentType(s.into()))
                };
                let bits = s[1..].parse::<usize>().map_err(|_| InstructionDefinitionError::BadArgumentType(s.into()))?;

                Ok(InstructionEncodingArgType::Immediate{signed, bits})
            }
        }
    }
}

impl TryFrom<String> for InstructionEncodingArgType {
    type Error = InstructionDefinitionError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::try_from(&s[..])
    }
}

impl InstructionEncodingArgType {
    pub fn bits(&self) -> usize {
        match self {
            InstructionEncodingArgType::Gpr => 3,
            InstructionEncodingArgType::ControlRegister => 3,
            InstructionEncodingArgType::Immediate { signed: _, bits } => *bits
        }
    }
}


#[derive(Deserialize, Debug)]
#[serde(from = "String")]
pub enum InstructionEncodingPiece {
    Literal(String),
    Ignored(usize),
    Arg(String)
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

#[derive(Error, Debug)]
pub enum InstructionDefinitionError {
    #[error("{0} is not a valid instruction argument type (should match `gpr|cr|[su][0-9]+`)")]
    BadArgumentType(String),

    #[error("{arg_name:?} is not an argument of instruction {mnemonic}")]
    UndefinedArgument { mnemonic: String, arg_name: String },

    #[error("Instruction {mnemonic} has bad encoding length")]
    WrongEncodingLength { mnemonic: String, bits: usize },
}

/// Helper for loading from json
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum OneOrMany<T> {
    One(T),
    Many(Vec<T>),
}

impl<T> From<OneOrMany<T>> for Vec<T> {
    fn from(value: OneOrMany<T>) -> Self {
        match value {
            OneOrMany::One(s) => vec![s],
            OneOrMany::Many(v) => v,
        }
    }
}

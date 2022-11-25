use num_enum::TryFromPrimitive;
use strum::EnumString;
#[cfg(test)]
use test_strategy::Arbitrary;
use thiserror::Error;
use ux::*; // Non-standard integer types

use crate::util::*;

pub type Word = u16;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Gpr(u3);

impl std::fmt::Display for Gpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "r{}", self.0)
    }
}

impl TryFrom<u16> for Gpr {
    type Error = <u3 as TryFrom<u16>>::Error;
    fn try_from(v: u16) -> Result<Self, Self::Error> {
        v.try_into().map(Gpr)
    }
}

impl From<Gpr> for usize {
    fn from(r: Gpr) -> Self {
        r.0.try_into().unwrap()
    }
}

impl Gpr {
    /// Create a new GPR index, panics if out of range
    pub fn new(i: usize) -> Gpr {
        Gpr(i.try_into().unwrap())
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, TryFromPrimitive, EnumString)]
#[cfg_attr(test, derive(Arbitrary))]
#[repr(u16)]
pub enum ControlRegister {
    AluStatus = 0,
    CpuStatus = 1,
    ContextID = 2,
    IntCause = 3,
    IntBase = 4,
    IntPc = 5,
    MMUAddr = 6,
    MMUData = 7,
}

impl std::fmt::Display for ControlRegister {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{:?}", self)
    }
}

#[derive(Copy, Clone, Debug, Error, PartialEq, Eq)]
pub enum InvalidInstructionError {
    #[error("Invalid instruction {0:#06x}")]
    InvalidOpcode(u16),
}

include!(concat!(env!("OUT_DIR"), "/instruction_def.rs"));

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[test]
    fn test_instruction_from_word_example1() {
        assert_eq!(
            Instruction::try_from(0u16).unwrap(),
            Instruction::Addi {
                r: Gpr::new(0),
                immediate: 0i8
            }
        )
    }

    #[test]
    fn test_instruction_from_word_example2() {
        assert_eq!(
            Instruction::try_from(0xffffu16).unwrap(),
            Instruction::Break
        )
    }

    #[test]
    fn test_instruction_from_word_invalid_opcode_example() {
        assert_eq!(
            Instruction::try_from(0xe000u16).unwrap_err(),
            InvalidInstructionError::InvalidOpcode(0xe000)
        );
    }

    #[test]
    fn test_instruction_display_example1() {
        assert_eq!(
            format!(
                "{}",
                Instruction::Ld {
                    rd: Gpr::new(3),
                    address: Gpr::new(4),
                    offset: i7::new(-14)
                }
            ),
            "ld r3, r4, -14"
        )
    }

    #[test]
    fn test_instruction_display_example2() {
        assert_eq!(
            format!(
                "{}",
                Instruction::Stcr {
                    cr: ControlRegister::CpuStatus,
                    rs: Gpr::new(7)
                }
            ),
            "stcr CpuStatus, r7"
        )
    }

    #[proptest]
    fn test_control_register_str_roundtrip(cr: ControlRegister) {
        use std::str::FromStr;
        let string = format!("{cr:?}");
        let converted = ControlRegister::from_str(&string).unwrap();
        assert_eq!(cr, converted);
    }

    #[test]
    fn test_control_register_bad_str() {
        use std::str::FromStr;
        ControlRegister::from_str("xxxxxxxxx").unwrap_err();
    }

    #[test]
    fn test_control_register_bad_str_lowercase() {
        use std::str::FromStr;
        let string = format!("{:?}", ControlRegister::CpuStatus).to_ascii_lowercase();
        ControlRegister::from_str(&string).unwrap_err();
    }
}

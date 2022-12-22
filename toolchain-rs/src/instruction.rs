use num_enum::TryFromPrimitive;
#[cfg(test)]
use proptest::{
    arbitrary::{any, Arbitrary},
    strategy::{BoxedStrategy, Strategy},
};
use std::str::FromStr;
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

impl From<Gpr> for u16 {
    fn from(r: Gpr) -> u16 {
        r.0.into()
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

impl FromStr for Gpr {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.strip_prefix("r")
            .and_then(|suffix| suffix.parse::<u16>().ok())
            .and_then(|n| Gpr::try_from(n).ok())
            .ok_or(())
    }
}

#[cfg(test)]
impl Arbitrary for Gpr {
    type Parameters = ();
    type Strategy = BoxedStrategy<Gpr>;

    fn arbitrary_with(_: ()) -> Self::Strategy {
        any::<u3>().prop_map(|x| Gpr(x)).boxed()
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

impl From<ControlRegister> for u16 {
    fn from(cr: ControlRegister) -> u16 {
        cr as u16
    }
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
    use test_case::test_case;
    use test_strategy::proptest;

    #[test_case(0x0000, Instruction::Addi { r: Gpr::new(0), immediate: 0i8 }; "addi")]
    #[test_case(0xffff, Instruction::Break; "brk")]
    fn instruction_from_word(num: u16, expected: Instruction) {
        assert_eq!(Instruction::try_from(num).unwrap(), expected)
    }

    #[test]
    fn instruction_from_word_invalid_opcode() {
        assert_eq!(
            Instruction::try_from(0xe000u16).unwrap_err(),
            InvalidInstructionError::InvalidOpcode(0xe000)
        );
    }

    #[test_case(Instruction::Ld { rd: Gpr::new(3), address: Gpr::new(4), offset: i7::new(-14) }, "ld r3, r4, -14"; "ld")]
    #[test_case(Instruction::Stcr { cr: ControlRegister::CpuStatus, rs: Gpr::new(7) }, "stcr CpuStatus, r7"; "stcr")]
    fn instruction_display_example1(instruction: Instruction, expected: &str) {
        assert_eq!(format!("{instruction}"), expected)
    }

    #[proptest]
    fn control_register_str_roundtrip(cr: ControlRegister) {
        let string = format!("{cr}");
        let converted = ControlRegister::from_str(&string).unwrap();
        assert_eq!(cr, converted);
    }

    #[test]
    fn control_register_bad_str() {
        ControlRegister::from_str("xxxxxxxxx").unwrap_err();
    }

    #[test]
    fn control_register_bad_str_lowercase() {
        let string = format!("{}", ControlRegister::CpuStatus).to_ascii_lowercase();
        ControlRegister::from_str(&string).unwrap_err();
    }

    #[proptest]
    fn control_register_u16_roundtrip(cr: ControlRegister) {
        let num: u16 = cr.into();
        let converted = ControlRegister::try_from(num).unwrap();
        assert_eq!(cr, converted);
    }

    #[proptest]
    fn control_register_bad_u16(#[strategy(8u16..)] num: u16) {
        ControlRegister::try_from(num).unwrap_err();
    }

    #[proptest]
    fn gpr_str_roundtrip(gpr: Gpr) {
        let string = format!("{gpr}");
        let converted = Gpr::from_str(&string).unwrap();
        assert_eq!(gpr, converted);
    }

    #[test_case("xxxx"; "completely_wrong")]
    #[test_case("rx"; "not_a_number")]
    #[test_case("r99"; "out_of_range")]
    #[test_case("s0"; "bad_prefix")]
    fn gpr_bad_str(s: &str) {
        Gpr::from_str(s).unwrap_err();
    }

    #[proptest]
    fn gpr_u16_roundtrip(gpr: Gpr) {
        let num: u16 = gpr.into();
        let converted = Gpr::try_from(num).unwrap();
        assert_eq!(gpr, converted);
    }

    #[proptest]
    fn gpr_bad_u16(#[strategy(8u16..)] num: u16) {
        Gpr::try_from(num).unwrap_err();
    }
}

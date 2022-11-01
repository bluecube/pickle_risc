use num_enum::TryFromPrimitive;
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, TryFromPrimitive)]
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

#[derive(Copy, Clone, Debug, Error)]
pub enum InvalidInstructionError {
    #[error("Invalid instruction {0:#06x}")]
    InvalidOpcode(u16),
}

include!(concat!(env!("OUT_DIR"), "/instruction_def.rs"));

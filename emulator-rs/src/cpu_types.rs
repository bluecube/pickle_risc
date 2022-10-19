//! Types used internally in the CPU definition

use std::fmt;

use ux::*; // Non-standard integer types
use num_enum::{TryFromPrimitive, IntoPrimitive};
use thiserror::Error;

use crate::util::*;

pub type Word = u16;

pub type Gpr = u3;

#[derive(Copy, Clone, Debug, TryFromPrimitive)]
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

#[derive(Copy, Clone, Debug)]
pub struct VirtualMemoryAddress {
    pub page_number: u6,
    pub offset: PageOffset
}

impl From<&VirtualMemoryAddress> for Word {
    fn from(v: &VirtualMemoryAddress) -> Self {
        Self::from(v.page_number) << PageOffset::BITS | Self::from(v.offset)
    }
}

impl From<Word> for VirtualMemoryAddress {
    fn from(v: Word) -> Self {
        VirtualMemoryAddress {
            page_number: (v >> PageOffset::BITS).try_into().unwrap(),
            offset: (v & Word::from(PageOffset::MAX)).try_into().unwrap()
        }
    }
}

impl fmt::Display for VirtualMemoryAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{:#06x}", Word::from(self))
    }
}

#[derive(Copy, Clone, Debug, IntoPrimitive, TryFromPrimitive)]
#[repr(u16)]
pub enum VirtualMemorySegment {
    Data = 0,
    Program = 1,
}

pub type ContextId = u6;
pub type PageNumber = u6;
pub type PageOffset = u10;

#[derive(Copy, Clone, Debug)]
pub struct PhysicalMemoryAddress {
    pub frame_number: u14,
    pub offset: PageOffset
}

impl From<&PhysicalMemoryAddress> for u32 {
    fn from(v: &PhysicalMemoryAddress) -> Self {
        Self::from(v.frame_number) << PageOffset::BITS | Self::from(v.offset)
    }
}

impl TryFrom<u32> for PhysicalMemoryAddress {
    type Error = <u14 as TryFrom<u32>>::Error;
    fn try_from(v: u32) -> Result<Self, Self::Error> {
        Ok(PhysicalMemoryAddress {
            frame_number: (v >> PageOffset::BITS).try_into()?,
            offset: (v & u32::from(PageOffset::MAX)).try_into().unwrap()
        })
    }
}

impl fmt::Display for PhysicalMemoryAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{:#09x}", u32::from(self))
    }
}

#[derive(Copy, Clone, Debug)]
pub struct PageTableIndex {
    pub context_id: u6,
    pub segment: VirtualMemorySegment,
    pub page_number: PageNumber,
}

impl PageTableIndex {
    pub const BITS: u32 = 7 + PageNumber::BITS;
}

impl From<&PageTableIndex> for Word {
    fn from(v: &PageTableIndex) -> Self {
        (Word::from(v.context_id) << (PageNumber::BITS + 1)) |
            (Word::from(v.segment) << PageNumber::BITS) |
            Word::from(v.page_number)
    }
}

impl From<&PageTableIndex> for usize {
    fn from(v: &PageTableIndex) -> Self {
        u16::from(v).into()
    }
}

impl TryFrom<Word> for PageTableIndex {
    type Error = <u6 as TryFrom<Word>>::Error;
    fn try_from(v: Word) -> Result<Self, Self::Error> {
        Ok(PageTableIndex {
            context_id: (v >> (PageNumber::BITS + 1)).try_into()?,
            segment: ((v >> PageNumber::BITS) & 1).try_into().unwrap(),
            page_number: (v & Word::from(PageNumber::MAX)).try_into().unwrap()
        })
    }
}

#[derive(Copy, Clone, Debug)]
pub struct PageTableRecord {
    pub readable: bool,
    pub writable: bool,
    pub frame_number: u14
}

impl From<&PageTableRecord> for Word {
    /// Converting PageTableRecord into the actual usize used for indexing the table
    fn from(v: &PageTableRecord) -> Self {
        ((v.readable as Word) << 15) |
            ((v.writable as Word) << 14) |
            Word::from(v.frame_number)
    }
}

impl From<Word> for PageTableRecord {
    /// Converting word written to control register to the page table index,
    /// used when the kernel code will fill the page table.
    fn from(v: Word) -> Self {
        PageTableRecord {
            readable: (v >> 15) & 1 != 0,
            writable: (v >> 14) & 1 != 0,
            frame_number: (v & Word::from(u14::MAX)).try_into().unwrap()
        }
    }
}

#[derive(Error,Debug)]
pub enum EmulatorError {
    #[error("Attempting to access non-mapped physical memory at {address} (pc = {pc})")]
    NonMappedPhysicalMemory { address: PhysicalMemoryAddress, pc: Word }
}

/*#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use test_strategy::proptest;

}*/

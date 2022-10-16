//! Types used internally in the CPU definition

use ux::*; // Non-standard integer types
use num_enum::{TryFromPrimitive, IntoPrimitive};
use thiserror::Error;

use crate::util::*;

pub type Word = u16;
pub type PhysicalMemoryAddress = u24;
pub type ContextId = u6;
pub type Gpr = u3;
pub type PageOffset = u10;

#[derive(Copy, Clone, Debug, IntoPrimitive, TryFromPrimitive)]
#[repr(u16)]
pub enum VirtualMemorySegment {
    Data = 0,
    Program = 1,
}

#[derive(Copy, Clone, Debug)]
pub struct VirtualMemoryAddress(pub u16, pub VirtualMemorySegment);

impl VirtualMemoryAddress {
    /// Split address into page number and offset within the page
    pub fn split_page_offset(&self) -> (u6, PageOffset) {
        (
            (self.0 >> PageOffset::BITS).try_into().expect("Should be limited by range of the input u16"),
            field!(self.0, PageOffset)
        )
    }
}

#[derive(Copy, Clone, Debug)]
pub struct PageTableIndex {
    pub context_id: u6,
    pub segment: VirtualMemorySegment,
    pub page_number: u6,
}

impl PageTableIndex {
    pub const BITS: u32 = 15;
    pub const MAX: usize = (1 << Self::BITS as usize) - 1;
}

impl From<PageTableIndex> for usize {
    /// Converting PageTableIndex into the actual usize used for indexing the table
    fn from(v: PageTableIndex) -> Self {
        let mut ret: Self = v.context_id.try_into().unwrap();

        ret <<= 1;
        ret |= usize::from(u16::from(v.segment));

        ret <<= 6;
        ret |= Self::try_from(v.page_number).unwrap();

        ret
    }
}

impl From<Word> for PageTableIndex {
    /// Converting word written to control register to the page table index,
    /// used when the kernel code will fill the page table.
    fn from(v: Word) -> Self {
        let mut vv = v;

        let page_number = field!(vv, u6);
        vv >>= 6;

        let segment: VirtualMemorySegment = (vv & 1).try_into().unwrap();
        vv >>= 1;

        let context_id = field!(vv, u6);

        PageTableIndex{ context_id, segment, page_number }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct PageTableRecord {
    pub readable: bool,
    pub writable: bool,
    pub frame_number: u14
}

impl From<Word> for PageTableRecord {
    /// Converting word written to control register to the page table record,
    /// used when the kernel code will fill the page table.
    fn from(v: Word) -> PageTableRecord {
        let mut vv = v;

        let frame_number = field!(vv, u14);
        vv >>= 14;

        let writable = (vv & 1) != 0;
        vv >>= 1;

        let readable = (vv & 1) != 0;

        PageTableRecord{ readable, writable, frame_number }
    }
}

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

impl ControlRegister {
    pub const BITS: u32 = 3;
}

#[derive(Error,Debug)]
pub enum EmulatorError {
    #[error("Attempting to access non-mapped physical memory at {address:#09x} (pc = {pc:#06x})")]
    NonMappedPhysicalMemory { address: PhysicalMemoryAddress, pc: Word }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use test_strategy::proptest;


}

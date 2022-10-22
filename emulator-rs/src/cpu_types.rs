//! Types used internally in the CPU definition

use std::fmt;

use ux::*; // Non-standard integer types
use num_enum::{TryFromPrimitive, IntoPrimitive};
use thiserror::Error;
#[cfg(test)] use test_strategy::Arbitrary;

pub type Word = u16;

pub type Gpr = u3;

#[derive(Copy, Clone, Debug,  PartialEq, Eq, TryFromPrimitive)]
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

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(test, derive(Arbitrary))]
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

#[derive(Copy, Clone, Debug,  PartialEq, Eq, IntoPrimitive, TryFromPrimitive)]
#[cfg_attr(test, derive(Arbitrary))]
#[repr(u16)]
pub enum VirtualMemorySegment {
    Data = 0,
    Program = 1,
}

pub type ContextId = u6;
pub type PageNumber = u6;
pub type PageOffset = u10;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct PhysicalMemoryAddress {
    pub frame_number: u14,
    pub offset: PageOffset
}

impl PhysicalMemoryAddress {
    const BITS: u32 = 14 + PageOffset::BITS;
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

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(test, derive(Arbitrary))]
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

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(test, derive(Arbitrary))]
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
    NonMappedPhysicalMemory { address: PhysicalMemoryAddress, pc: Word },
    #[error("Instruction `{mnemonic} has no microcode defined (TODO) (pc = {pc})")]
    MissingMicrocode {mnemonic: &'static str , pc: Word },
    #[error("Error when accessing memory")]
    MemoryAccessError { pc: Word },
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;
    use more_asserts::*;

    #[test]
    fn test_virtual_memory_address_from_word_example() {
        let a = VirtualMemoryAddress::from(0b101010__1100110011);
        assert_eq!(u16::from(a.page_number), 0b101010);
        assert_eq!(u16::from(a.offset), 0b1100110011);
    }

    #[proptest]
    fn test_virtual_memory_address_roundtrip1(a: VirtualMemoryAddress) {
        assert_eq!(VirtualMemoryAddress::from(Word::from(&a)), a);
    }

    #[proptest]
    fn test_virtual_memory_address_roundtrip2(a: u16) {
        assert_eq!(Word::from(&VirtualMemoryAddress::from(a)), a);
    }

    #[test]
    fn test_physical_memory_address_from_word_example() {
        let a = PhysicalMemoryAddress::try_from(0b10101010101010__1100110011).unwrap();
        assert_eq!(u16::from(a.frame_number), 0b10101010101010);
        assert_eq!(u16::from(a.offset), 0b1100110011);
    }

    #[proptest]
    fn test_physical_memory_address_roundtrip1(a: PhysicalMemoryAddress) {
        assert_eq!(PhysicalMemoryAddress::try_from(u32::from(&a)).unwrap(), a);
    }

    #[proptest]
    fn test_physical_memory_address_bits(a: PhysicalMemoryAddress) {
        assert_le!(u32::from(&a).next_power_of_two(), 1 << PhysicalMemoryAddress::BITS);
    }

    #[proptest]
    fn test_physical_memory_address_roundtrip2(
        #[strategy(0u32 .. 1u32 << PhysicalMemoryAddress::BITS)]
        a: u32
    ) {
        assert_eq!(u32::from(&PhysicalMemoryAddress::try_from(a).unwrap()), a);
    }

    #[proptest]
    fn test_physical_memory_address_out_of_range(
        #[strategy((1u32 << PhysicalMemoryAddress::BITS ..= u32::MAX))]
        a: u32
    ) {
        PhysicalMemoryAddress::try_from(a).unwrap_err();
    }

    #[test]
    fn test_page_table_index_from_word_example() {
        let i = PageTableIndex::try_from(0b111000_1_110011).unwrap();
        assert_eq!(u16::from(i.context_id), 0b111000);
        assert_eq!(i.segment, VirtualMemorySegment::Program);
        assert_eq!(u16::from(i.page_number), 0b110011);
    }

    #[proptest]
    fn test_page_table_index_roundtrip1(i: PageTableIndex) {
        assert_eq!(PageTableIndex::try_from(Word::from(&i)).unwrap(), i);
    }

    #[proptest]
    fn test_page_table_index_bits(i: PageTableIndex) {
        assert_le!(u16::from(&i).next_power_of_two(), 1 << PageTableIndex::BITS);
    }

    #[proptest]
    fn test_page_table_index_roundtrip2(
        #[strategy(0u16 .. 1u16 << PageTableIndex::BITS)]
        a: u16
    ) {
        assert_eq!(u16::from(&PageTableIndex::try_from(a).unwrap()), a);
    }

    #[proptest]
    fn test_page_table_index_out_of_range(
        #[strategy((1u16 << PageTableIndex::BITS ..= u16::MAX))]
        i: u16
    ) {
        PageTableIndex::try_from(i).unwrap_err();
    }

    #[test]
    fn test_page_table_record_from_word_example() {
        let r = PageTableRecord::try_from(0b1_0_11001100110011).unwrap();
        assert!(r.readable);
        assert!(!r.writable);
        assert_eq!(u16::from(r.frame_number), 0b11001100110011);
    }

    #[proptest]
    fn test_page_table_record_roundtrip1(r: PageTableRecord) {
        assert_eq!(PageTableRecord::from(Word::from(&r)), r);
    }

    #[proptest]
    fn test_page_table_record_roundtrip2(a: u16) {
        assert_eq!(Word::from(&PageTableRecord::from(a)), a);
    }
}
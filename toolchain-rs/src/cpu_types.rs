//! Types used internally in the CPU definition

use std::fmt;

use num_enum::{IntoPrimitive, TryFromPrimitive};
#[cfg(test)]
use test_strategy::Arbitrary;
use thiserror::Error;
use ux::*;

pub use crate::instruction::{ControlRegister, Gpr, Word};

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct CpuStatus {
    pub interrupt_enabled: bool,
    pub kernel_mode: bool,
    pub mmu_enabled: bool,
}

impl CpuStatus {
    const MASK: Word = 0x0007;
    const BITS: Word = Self::MASK.count_ones() as u16;
}

impl From<&CpuStatus> for Word {
    fn from(v: &CpuStatus) -> Self {
        (v.interrupt_enabled as Word) | (v.kernel_mode as Word) << 1 | (v.mmu_enabled as Word) << 2
    }
}

impl TryFrom<Word> for CpuStatus {
    type Error = EmulatorError;
    fn try_from(v: Word) -> Result<Self, Self::Error> {
        if v & !Self::MASK != 0 {
            Err(EmulatorError::ReservedBitNonzero {
                t: "CpuStatus".into(),
                value: v,
            })
        } else {
            Ok(CpuStatus {
                interrupt_enabled: v & 1 != 0,
                kernel_mode: (v >> 1) & 1 != 0,
                mmu_enabled: (v >> 2) & 1 != 0,
            })
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct VirtualMemoryAddress {
    pub page_number: u6,
    pub offset: PageOffset,
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
            offset: (v & Word::from(PageOffset::MAX)).try_into().unwrap(),
        }
    }
}

impl fmt::Display for VirtualMemoryAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{:#06x}", Word::from(self))
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, IntoPrimitive, TryFromPrimitive)]
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
    pub offset: PageOffset,
}

impl From<&PhysicalMemoryAddress> for u24 {
    fn from(v: &PhysicalMemoryAddress) -> Self {
        Self::from(v.frame_number) << PageOffset::BITS | Self::from(v.offset)
    }
}

impl From<u24> for PhysicalMemoryAddress {
    fn from(v: u24) -> Self {
        PhysicalMemoryAddress {
            frame_number: (v >> PageOffset::BITS).try_into().unwrap(),
            offset: (v & u24::from(PageOffset::MAX)).try_into().unwrap(),
        }
    }
}

impl fmt::Display for PhysicalMemoryAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{:#09x}", u32::from(u24::from(self)))
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
        (Word::from(v.context_id) << (PageNumber::BITS + 1))
            | (Word::from(v.segment) << PageNumber::BITS)
            | Word::from(v.page_number)
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
            page_number: (v & Word::from(PageNumber::MAX)).try_into().unwrap(),
        })
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct PageTableRecord {
    pub readable: bool,
    pub writable: bool,
    pub frame_number: u14,
}

impl From<&PageTableRecord> for Word {
    /// Converting PageTableRecord into the actual usize used for indexing the table
    fn from(v: &PageTableRecord) -> Self {
        ((v.readable as Word) << 15) | ((v.writable as Word) << 14) | Word::from(v.frame_number)
    }
}

impl From<Word> for PageTableRecord {
    /// Converting word written to control register to the page table index,
    /// used when the kernel code will fill the page table.
    fn from(v: Word) -> Self {
        PageTableRecord {
            readable: (v >> 15) & 1 != 0,
            writable: (v >> 14) & 1 != 0,
            frame_number: (v & Word::from(u14::MAX)).try_into().unwrap(),
        }
    }
}

#[derive(Error, Debug)]
pub enum EmulatorError {
    #[error("Attempting to access non-mapped physical memory at {address} (pc = {pc:#06x})")]
    NonMappedPhysicalMemory {
        address: PhysicalMemoryAddress,
        pc: Word,
    },
    #[error("Instruction `{mnemonic} has no microcode defined (TODO) (pc = {pc:#06x})")]
    MissingMicrocode { mnemonic: &'static str, pc: Word },
    #[error("Error when accessing memory")]
    MemoryAccessError { pc: Word },
    #[error("Reserved bit position written as nonzero when writing {t} (value: {value:#06x})")]
    ReservedBitNonzero { t: String, value: Word },
}

#[cfg(test)]
mod tests {
    use super::*;
    use more_asserts::*;
    use test_strategy::proptest;

    #[proptest]
    fn test_cpu_status_roundtrip1(s: CpuStatus) {
        assert_eq!(CpuStatus::try_from(Word::from(&s)).unwrap(), s);
    }

    #[proptest]
    fn test_cpu_status_mask(s: CpuStatus) {
        assert_eq!(Word::from(&s) & !CpuStatus::MASK, 0);
    }

    #[proptest]
    fn test_cpu_status_roundtrip2(#[strategy(0u16 .. 1u16 << CpuStatus::BITS)] s: Word) {
        assert!(
            s & !CpuStatus::MASK == 0,
            "This test is fragile w.r.t. the gaps in bitmask, fix it if this assertion fails"
        );
        assert_eq!(Word::from(&CpuStatus::try_from(s).unwrap()), s);
    }

    #[proptest]
    fn test_cpu_status_out_of_range(#[strategy((1u16 << CpuStatus::BITS ..= u16::MAX))] s: u16) {
        assert!(
            s & !CpuStatus::MASK != 0,
            "This test is fragile w.r.t. the gaps in bitmask, fix it if this assertion fails"
        );
        CpuStatus::try_from(s).unwrap_err();
    }

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
        let a = PhysicalMemoryAddress::from(u24::new(0b10101010101010__1100110011));
        assert_eq!(u16::from(a.frame_number), 0b10101010101010);
        assert_eq!(u16::from(a.offset), 0b1100110011);
    }

    #[proptest]
    fn test_physical_memory_address_roundtrip1(a: PhysicalMemoryAddress) {
        assert_eq!(PhysicalMemoryAddress::from(u24::from(&a)), a);
    }

    #[proptest]
    fn test_physical_memory_address_roundtrip2(a: u24) {
        assert_eq!(u24::from(&PhysicalMemoryAddress::from(a)), a);
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
    fn test_page_table_index_roundtrip2(#[strategy(0u16 .. 1u16 << PageTableIndex::BITS)] a: u16) {
        assert_eq!(u16::from(&PageTableIndex::try_from(a).unwrap()), a);
    }

    #[proptest]
    fn test_page_table_index_out_of_range(
        #[strategy((1u16 << PageTableIndex::BITS ..= u16::MAX))] i: u16,
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

use ux::*; // Non-standard integer types

use crate::cpu_types::*;
use crate::instruction::Opcode;
use crate::util::*;

const PAGE_TABLE_SIZE: usize = 1 << PageTableIndex::BITS;

#[derive(Debug)]
pub struct CpuState {
    gpr: [Word; 7],

    pc: Word,

    alu_flags: Word,
    cpu_status: CpuStatus,
    context_id: ContextId,
    int_c: Word,
    int_cause: Word,
    mmu_addr: Word,

    // Microinstruction step
    step: u2,
    current_instruction: Word,
    next_instruction: Word,

    page_table: [PageTableRecord; PAGE_TABLE_SIZE],
}

#[cfg_attr(feature = "cargo-clippy", allow(clippy::new_without_default))]
impl CpuState {
    /// Construct new CPU state in a reset state, with everything zero filled.
    pub fn new() -> Self {
        // This doesn't need to be a valid initial state!
        // (although likely it is)
        let mut ret = CpuState {
            gpr: [0; 7],

            pc: 0,

            alu_flags: 0,
            cpu_status: CpuStatus::default(),
            context_id: u6::new(0),
            int_c: 0,
            int_cause: 0,
            mmu_addr: 0,

            step: u2::new(0),
            current_instruction: 0,
            next_instruction: 0,
            page_table: [PageTableRecord::default(); PAGE_TABLE_SIZE],
        };
        // Call reset to make sure we are in a valid startup state
        ret.reset();
        ret
    }

    /// Construct new CPU in a reset state, but otherwise completely random
    pub fn with_rng(rng: &mut impl rand::Rng) -> Self {
        let mut ret = CpuState {
            gpr: rng.gen(),

            pc: rng.gen(),

            alu_flags: rng.gen(),
            cpu_status: (rng.gen::<Word>() & CpuStatus::MASK).try_into().unwrap(),
            context_id: u6::new(rng.gen_range(0u8..=u6::MAX.into())),
            int_c: rng.gen(),
            int_cause: rng.gen(),
            mmu_addr: rng.gen(),

            step: u2::new(rng.gen_range(0u8..=u2::MAX.into())),
            current_instruction: rng.gen(),
            next_instruction: rng.gen(),
            page_table: [PageTableRecord::default(); PAGE_TABLE_SIZE],
        };

        for i in 0..PAGE_TABLE_SIZE {
            ret.page_table[i] = rng.gen::<Word>().into();
        }

        // Call reset to make sure we are in a valid startup state
        ret.reset();
        ret
    }

    /// Reset the bare minimum of state to reboot the computer.
    pub fn reset(&mut self) {
        self.pc = 0;
        self.cpu_status = CpuStatus {
            interrupt_enabled: false,
            kernel_mode: false,
            mmu_enabled: false,
        };
        self.step = u2::default();
        self.current_instruction = 0;
    }

    pub fn get_next_instruction(&self) -> Word {
        self.next_instruction
    }

    pub fn get_pc(&self) -> Word {
        self.pc
    }

    pub fn get_step(&self) -> u2 {
        self.step
    }

    pub fn get_gpr(&self, index: Gpr) -> Word {
        let i: usize = index.into();
        if i == 0 {
            0
        } else {
            self.gpr[i - 1]
        }
    }

    pub fn set_gpr(&mut self, index: Gpr, value: Word) {
        let i: usize = index.into();
        if i > 0 {
            self.gpr[i - 1] = value
        }
    }

    pub fn get_cr(&self, index: ControlRegister) -> Word {
        todo!("get value of control register {:?}", index);
    }

    pub fn set_cr(&mut self, index: ControlRegister, value: Word) {
        todo!("set value of control register {:?} to {}", index, value);
    }

    fn read_memory<M: PhysicaMemory>(
        &mut self,
        address: &VirtualMemoryAddress,
        segment: &VirtualMemorySegment,
        memory: &M,
    ) -> Result<Word, EmulatorError> {
        self.memory_operation(address, segment, false, |a| memory.read(a))
    }

    fn write_memory<M: PhysicaMemory>(
        &mut self,
        address: &VirtualMemoryAddress,
        segment: &VirtualMemorySegment,
        memory: &mut M,
        value: Word,
    ) -> Result<(), EmulatorError> {
        self.memory_operation(address, segment, true, |a| memory.write(a, value))
    }

    /// Helper that combines common operations when accessing memory
    /// `fun` contains the actual memory operation, it is given the effective address as u24.
    /// `fun` might return None if the (physical) memory is not mapped for this address.
    fn memory_operation<R: Default, F: FnOnce(u24) -> Option<R>>(
        &mut self,
        address: &VirtualMemoryAddress,
        segment: &VirtualMemorySegment,
        write: bool,
        fun: F,
    ) -> Result<R, EmulatorError> {
        if let Some(physical_address) = self.virtual_to_physical(address, segment, write) {
            let address: u24 = (&physical_address).into();
            fun(address).ok_or(EmulatorError::NonMappedPhysicalMemory {
                address: physical_address,
                pc: self.pc,
            })
        } else {
            self.page_fault();
            Ok(R::default())
        }
    }

    fn page_fault(&mut self) {
        todo!("Page fault")
    }

    fn virtual_to_physical(
        &self,
        address: &VirtualMemoryAddress,
        segment: &VirtualMemorySegment,
        write: bool,
    ) -> Option<PhysicalMemoryAddress> {
        if self.cpu_status.mmu_enabled {
            let page_table_index = PageTableIndex {
                context_id: self.context_id,
                segment: *segment,
                page_number: address.page_number,
            };

            let page = self.page_table[usize::from(&page_table_index)];

            if (write && !page.writable) | (!write && !page.readable) {
                None
            } else {
                Some(PhysicalMemoryAddress {
                    frame_number: page.frame_number,
                    offset: address.offset,
                })
            }
        } else {
            Some(PhysicalMemoryAddress {
                frame_number: match segment {
                    VirtualMemorySegment::Data => u14::new(0),
                    VirtualMemorySegment::Program => u14::new(1 << 13),
                },
                offset: address.offset,
            })
        }
    }

    fn write_memory_mapping(&mut self, page_table_index: &PageTableIndex, record: PageTableRecord) {
        self.page_table[usize::from(page_table_index)] = record;
    }

    fn end_instruction(&mut self) {
        self.current_instruction = self.next_instruction;
        self.step = u2::default();
    }

    pub fn step<M: PhysicaMemory>(&mut self, memory: &M) -> Result<(), EmulatorError> {
        let opcode = self.current_instruction;
        include!(concat!(env!("OUT_DIR"), "/microcode_def.rs"));
        Ok(())
    }
}

/// Represents a chunk of physical memory of 16bit values, accessable by 24bit address
/// Read and write might panic if address is > max_address().
/// Read and write can return None if the access is not mapped for some reason (eg. writing ROM)
pub trait PhysicaMemory {
    fn max_address(&self) -> u24;
    fn read(&self, address: u24) -> Option<Word>;
    fn write(&mut self, address: u24, value: Word) -> Option<()>;
}

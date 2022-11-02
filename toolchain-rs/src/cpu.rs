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

    current_instruction: Word,
    next_instruction: Word,

    page_table: [PageTableRecord; PAGE_TABLE_SIZE],
}

#[cfg_attr(feature = "cargo-clippy", allow(clippy::new_without_default))]
impl CpuState {
    /// Construct new CPU state in a reset state
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

            current_instruction: 0,
            next_instruction: 0,
            page_table: [PageTableRecord::default(); PAGE_TABLE_SIZE],
        };
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
        self.context_id = u6::new(0); // TODO: Is this necessary?
        self.current_instruction = 0;

        // TODO: Disable MMU and interrupts
    }

    pub fn get_next_instruction(&self) -> Word {
        // TODO: This should return the instruction disassembled!
        self.next_instruction
    }

    pub fn get_pc(&self) -> Word {
        self.pc
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
    ) -> anyhow::Result<Word> {
        self.memory_operation(address, segment, false, |a| memory.read(a))
    }

    fn write_memory<M: PhysicaMemory>(
        &mut self,
        address: &VirtualMemoryAddress,
        segment: &VirtualMemorySegment,
        memory: &mut M,
        value: Word,
    ) -> anyhow::Result<()> {
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
    ) -> anyhow::Result<R> {
        if let Some(physical_address) = self.virtual_to_physical(address, segment, write) {
            let address: u24 = (&physical_address).into();
            fun(address).ok_or_else(||
                EmulatorError::NonMappedPhysicalMemory {
                    address: physical_address,
                    pc: self.pc,
                }
                .into(),
            )
        } else {
            self.page_fault()?;
            Ok(R::default())
        }
    }

    fn page_fault(&mut self) -> anyhow::Result<()> {
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

    pub fn step<M: PhysicaMemory>(&mut self, memory: &M) -> anyhow::Result<()> {
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

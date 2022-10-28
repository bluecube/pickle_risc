use std::path::Path;
use ux::*; // Non-standard integer types

use crate::cpu::{CpuState, PhysicaMemory};
use crate::cpu_types::Word;
use crate::memory::{Ram, Rom};

/// Represents the state of the whole computer, including memories and peripherials.
/// This is a fixed implementation, that is supposed to closely match the planned hardware.
/// Potentially a dynamic one could be implemented, but right now I don't see why :)
pub struct SystemState {
    pub cpu: CpuState,
    pub devices: SystemDevices,
}

/// All devices that are mapped in memory
pub struct SystemDevices {
    pub ram: Ram,
    pub rom: Rom,
}

impl SystemState {
    pub fn new<P: AsRef<Path>>(rom_ihex_path: P) -> anyhow::Result<SystemState> {
        Ok(SystemState {
            cpu: CpuState::new(),
            devices: SystemDevices {
                ram: Ram::new(1 * 1024 * 1024), // 1MW to start
                rom: Rom::from_ihex(rom_ihex_path)?,
            },
        })
    }

    pub fn reset(&mut self) {
        self.cpu.reset();
    }

    pub fn step(&mut self) -> anyhow::Result<()> {
        self.cpu.step(&self.devices)?;
        self.devices.step()?;
        Ok(())
    }
}

impl PhysicaMemory for SystemDevices {
    // Memory is mapped as follows:
    // 0x000000 - 0x7fffff - RAM
    // 0x800000 - 0x8fffff - ROM ( = device 0)
    // 0x900000 - 0x9fffff - Device 1
    // 0x900000 - 0x9fffff - Device 2
    // ...
    // 0xf00000 - 0xffffff - Device 15

    fn max_address(&self) -> u24 {
        u24::MAX // no part of memory will panic, although some parts are not mapped
    }

    fn read(&self, address: u24) -> Option<Word> {
        if address <= self.ram.max_address() {
            self.ram.read(address)
        } else {
            let device_id = u32::from(address >> 20);
            let device_address = address & u24::new(0x0fffff);
            match device_id {
                0 if device_address <= self.rom.max_address() => self.rom.read(device_address),
                _ => None,
            }
        }
    }

    fn write(&mut self, address: u24, value: Word) -> Option<()> {
        if address <= self.ram.max_address() {
            self.ram.write(address, value)
        } else {
            None
            /*let device_id = u32::from(address >> 20);
            let device_address = address & u24::new(0x0fffff);
            match device_id {
                0 if device_address <= self.rom.max_address() => self.rom.write(device_address, value),
                _ => None
            }*/
        }
    }
}

impl SystemDevices {
    pub fn step(&mut self) -> anyhow::Result<()> {
        // No devices need stepping at the moment
        Ok(())
    }
}

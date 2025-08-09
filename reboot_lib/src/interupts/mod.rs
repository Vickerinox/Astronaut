use crate::MemoryWrapper;
use volatile_register::*;

pub const INTERUPT_HARDWARE: MemoryWrapper<InteruptRegisters> = MemoryWrapper(0x4000208 as *mut InteruptRegisters);

#[repr(C)]
pub struct InteruptRegisters {
    pub master: RW<u32>,
    _unused: u32,
    pub enable: RW<u32>,
    pub request: RW<u32>,
}
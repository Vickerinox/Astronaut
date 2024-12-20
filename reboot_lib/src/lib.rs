#![no_std]
#![feature(allocator_api)]
#![feature(vec_into_raw_parts)]
extern crate alloc;
mod allocator;
mod crypto;
mod fs;
mod ipc;
mod memory;
mod mmc;
mod ndma;
mod spi;
mod swi;
mod video;

pub use allocator::ALLOCATOR;
pub use crypto::*;
pub use ipc::IPC_FIFO_HARDWARE;
pub use mmc::driver::*;
pub use mmc::tmio::*;
pub use mmc::*;
pub use video::*;

pub struct RegisterWrapper<T>(*mut T);
impl<T> core::ops::Deref for RegisterWrapper<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0 }
    }
}
impl<T> core::ops::DerefMut for RegisterWrapper<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.0 }
    }
}

//master interrupt enable register.
const REG_IME: *mut u32 = 0x4000208 as *mut u32;
pub unsafe fn critical_function<F: FnOnce()>(closure: F) {
    let mut ime = 0;
    core::ptr::swap(REG_IME, &mut ime);
    closure();
    core::ptr::swap(REG_IME, &mut ime);
}

#[repr(u8)]
pub enum Command {
    ReadController = 0,
    ReadSDSector = 1,
    WriteSDSector = 2,
    ReadNANDSector = 3,
    WriteNANDSector = 4,
    Ready = 5,
    Busy = 6,
    Error = 7,
    Abort = 8,
}

pub enum ParsedCommand {
    ReadController(u32),
    ReadSDSector(*mut u8, u32, u32),
    WriteSDSector(*mut u8, u32, u32),
    ReadNANDSector(*mut u8, u32, u32),
    WriteNANDSector(*mut u8, u32, u32),
}

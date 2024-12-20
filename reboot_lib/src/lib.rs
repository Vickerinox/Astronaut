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
    ReadRegister = 0,
    ReadSDSector = 1,
    WriteSDSector = 2,
    ReadNANDSector = 3,
    WriteNANDSector = 4,
}
#[repr(u8)]
pub enum Response {
    Ready = 0,
    Ok = 1,
    Error = 2,
}
bitflags::bitflags! {
    pub struct Buttons: u16 {
        const BUTTON_A = (1 << 0);
        const BUTTON_B = (1 << 1);
        const BUTTON_SELECT = (1 << 2);
        const BUTTON_START = (1 << 3);
        const DIRECTION_RIGHT = (1 << 4);
        const DIRECTION_LEFT = (1 << 5);
        const DIRECTION_UP = (1 << 6);
        const DIRECTION_DOWN = (1 << 7);
        const BUTTON_R = (1 << 8);
        const BUTTON_L = (1 << 9);
        const BUTTON_X = (1 << 10);
        const BUTTON_Y = (1 << 11);
        const BUTTON_DEBUG = (1 << 12);
        const PEN_DOWN = (1 << 13);
        const LID_DOWN = (1 << 14);

    }
}

#[repr(C)]
pub struct Controls {
    buttons: Buttons,
    touch_x: u8,
    touch_y: u8,
}
pub unsafe fn arm9_send_controller_read() {
    IPC_FIFO_HARDWARE.set_status(1);
    IPC_FIFO_HARDWARE.send_raw_blocking(0);
    while IPC_FIFO_HARDWARE.read_status() != 1 {}
    IPC_FIFO_HARDWARE.set_status(0);
} 
pub unsafe fn arm9_set_buffer(slice: *mut [StorageSector]) {
    IPC_FIFO_HARDWARE.set_status(2);
    IPC_FIFO_HARDWARE.send_raw_blocking(slice as *mut StorageSector as u32);
    IPC_FIFO_HARDWARE.send_raw_blocking(slice.len() as u32);
    while IPC_FIFO_HARDWARE.read_status() != 1 {}
    IPC_FIFO_HARDWARE.set_status(0);
}
pub unsafe fn arm9_read_nand_sector_encrypted(start_sector: u32) {
    IPC_FIFO_HARDWARE.set_status(3);
    IPC_FIFO_HARDWARE.send_raw_blocking(start_sector);
    while IPC_FIFO_HARDWARE.read_status() != 1 {}
    IPC_FIFO_HARDWARE.set_status(0);
}
pub unsafe fn arm9_read_nand_sector_unencrypted(start_sector: u32) {
    IPC_FIFO_HARDWARE.set_status(4);
    IPC_FIFO_HARDWARE.send_raw_blocking(start_sector);
    while IPC_FIFO_HARDWARE.read_status() != 1 {}
    IPC_FIFO_HARDWARE.set_status(0);
}
pub unsafe fn arm9_read_sd_sector(start_sector: u32) {
    IPC_FIFO_HARDWARE.set_status(5);
    IPC_FIFO_HARDWARE.send_raw_blocking(start_sector);
    while IPC_FIFO_HARDWARE.read_status() != 1 {}
    IPC_FIFO_HARDWARE.set_status(0);
}
pub type StorageSector = [u32; 128];

#![no_std]
#![feature(allocator_api)]
#![feature(vec_into_raw_parts)]
#![feature(ptr_metadata)]
#![allow(unused)]
extern crate alloc;
mod allocator;
mod crypto;
mod fs;
pub mod i2c;
pub mod interupts;
mod ipc;
mod memory;
pub mod mmc;
pub mod ndma;
pub mod sound;
pub mod spi;
mod swi;
mod video;

pub use allocator::ALLOCATOR;
pub use crypto::*;
pub use interupts::*;
pub use ipc::IPC_FIFO_HARDWARE;
pub use mmc::driver::*;
pub use mmc::tmio::*;
pub use mmc::*;
pub use swi::*;
pub use video::*;
pub struct MemoryWrapper<T>(*mut T);
impl<T> core::ops::Deref for MemoryWrapper<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0 }
    }
}
impl<T> core::ops::DerefMut for MemoryWrapper<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.0 }
    }
}

//master interrupt enable register.
const REG_IME: *mut u32 = 0x4000208 as *mut u32;
pub unsafe fn critical_function<F: FnOnce()>(closure: F) {
    let mut ime = REG_IME.read_volatile();
    REG_IME.write_volatile(0);

    closure();
    REG_IME.write_volatile(ime);
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
    #[derive(Clone, Copy, PartialEq)]
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
unsafe fn com_arm9(opcode: u8, data_out: &[u32]) -> Result<(), u32> {
    IPC_FIFO_HARDWARE.send_raw_blocking(opcode as u32);
    for data in data_out.into_iter().copied() {
        IPC_FIFO_HARDWARE.send_raw_blocking(data);
    }

    let value = IPC_FIFO_HARDWARE.recieve_raw_blocking();
    assert!(IPC_FIFO_HARDWARE.recieve_value_raw().is_err());
    if value != 0 {
        Err(value)
    } else {
        Ok(())
    }
}
pub unsafe fn arm9_send_controller_read() -> Buttons {
    let value = com_arm9(1, &[0]).err().unwrap_or(0);
    Buttons::from_bits_retain(value as u16)
}
pub unsafe fn arm9_set_buffer(slice: *mut [StorageSector]) -> Result<(), u32> {
    com_arm9(2, &[slice as *mut () as u32, slice.len() as u32])
}
pub unsafe fn arm9_read_nand_sector_encrypted(start_sector: u32) -> Result<(), u32> {
    com_arm9(3, &[start_sector])
}
pub unsafe fn arm9_read_nand_sector_unencrypted(start_sector: u32) -> Result<(), u32> {
    com_arm9(4, &[start_sector])
}
pub unsafe fn arm9_read_sd_sector(start_sector: u32) -> Result<(), u32> {
    com_arm9(5, &[start_sector])
}
pub unsafe fn arm9_send_arm7_jump(ptr: u32) -> Result<(), u32> {
    com_arm9(6, &[ptr])
}
pub unsafe fn arm9_read_firmware(start_address: u32) -> Result<(), u32> {
    com_arm9(7, &[start_address])
}
pub unsafe fn arm9_ready_arm7() -> Result<(), u32> {
    com_arm9(8, &[0xB00B135])
}
pub struct StorageSector([u32; 128]);

impl AsMut<[u8]> for StorageSector {
    fn as_mut(&mut self) -> &mut [u8] {
        unsafe {
            &mut *core::ptr::from_raw_parts_mut(self as *mut Self as *mut u8, size_of::<Self>())
        }
    }
}
impl AsMut<[u32]> for StorageSector {
    fn as_mut(&mut self) -> &mut [u32] {
        &mut self.0[..]
    }
}
impl AsRef<[u32]> for StorageSector {
    fn as_ref(&self) -> &[u32] {
        &self.0[..]
    }
}
impl StorageSector {
    pub fn bytes(&self) -> &[u8] {
        unsafe { &*core::ptr::from_raw_parts(self as *const Self as *const u8, size_of::<Self>()) }
    }
}

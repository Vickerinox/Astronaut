#![no_std]
#![feature(allocator_api)]
#![feature(ptr_metadata)]
#![allow(unused)]
extern crate alloc;

#[macro_export]
macro_rules! const_assert {
    ($($tt:tt)*) => {
        const _: () = assert!($($tt)*);
    }
}
pub use volatile_register;
mod aes;
mod allocator;
pub mod autoboot_info;
pub mod dma;
pub mod standard_arm7;
mod fs;
pub mod i2c;
pub mod interupts;
mod ipc;
pub mod mbk;
mod memory;
pub mod mmc;
pub mod music_modules;
pub mod ndma;
pub mod scfg;
pub mod sound;
pub mod spi;
mod swi;
pub mod timers;
mod video;
pub use bitflags;
pub mod rtc;
use core::num::NonZeroU32;
pub mod twl_wifi;

pub use aes::*;
pub use allocator::ALLOCATOR;
pub use dma::*;
pub use interupts::*;
pub use ipc::IPC_FIFO_HARDWARE;
pub use memory::VRAMCtrl;
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
pub unsafe fn nocash_write(str: &str) {
    nocash_write_bytes(str.as_bytes());
}
pub unsafe fn nocash_str(str: &str) {
    (0x4fffa10 as *mut u32).write(core::ptr::addr_of!(*str) as *const u8 as usize as u32);
}
pub unsafe fn nocash_write_bytes(str: &[u8]) {
    const NOCASH_OUT_CHR: *mut u8 = 0x4fffa1c as *mut u8;
    for byte in str {
        NOCASH_OUT_CHR.write_volatile(*byte);
    }
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
unsafe fn com_arm9(opcode: u8, data_out: &[u32]) -> Result<(), NonZeroU32> {
    IPC_FIFO_HARDWARE.send_raw_blocking(opcode as u32);
    for data in data_out.into_iter().copied() {
        IPC_FIFO_HARDWARE.send_raw_blocking(data);
    }
    loop {
        if let Ok(value) = IPC_FIFO_HARDWARE.recieve_value_raw() {
            assert!(IPC_FIFO_HARDWARE.recieve_value_raw().is_err());
            match NonZeroU32::new(value) {
                Some(value) => return Err(value),
                None => return Ok(()),
            }
        } else if IPC_FIFO_HARDWARE.read_status() == 7 {
            panic!("ARM7 crashed while sending command {opcode}");
        }
    }
}
pub unsafe fn arm9_send_controller_read() -> (Buttons, u8, u8) {
    let value = com_arm9(1, &[0])
        .map_err(|i| u32::from(i))
        .err()
        .unwrap_or(0);
    (
        Buttons::from_bits_retain(value as u16),
        (value >> 16) as u8,
        (value >> 24) as u8,
    )
}
pub unsafe fn arm9_set_buffer(slice: *mut [StorageSector]) -> Result<(), NonZeroU32> {
    let (ptr, len) = slice.to_raw_parts();
    com_arm9(2, &[ptr as u32, len as u32])
}
pub unsafe fn arm9_read_nand_sector_encrypted(start_sector: u32) -> Result<(), NonZeroU32> {
    com_arm9(3, &[start_sector])
}
pub unsafe fn arm9_read_nand_sector_unencrypted(start_sector: u32) -> Result<(), NonZeroU32> {
    com_arm9(4, &[start_sector])
}
pub unsafe fn arm9_read_sd_sector(start_sector: u32) -> Result<(), NonZeroU32> {
    com_arm9(5, &[start_sector])
}
pub unsafe fn arm9_write_sd_sector(start_sector: u32) -> Result<(), NonZeroU32> {
    com_arm9(10, &[start_sector])
}
pub unsafe fn arm9_send_arm7_jump(ptr: u32) -> Result<(), NonZeroU32> {
    com_arm9(6, &[ptr])
}
pub unsafe fn arm9_read_firmware(start_address: u32) -> Result<(), NonZeroU32> {
    com_arm9(7, &[start_address])
}
pub unsafe fn arm9_decrypt_modcrypt(header: u32) -> Result<(), NonZeroU32> {
    com_arm9(12, &[header])
}

pub unsafe fn arm9_send_arm7(user_type: u32, pointer: *mut ()) -> Result<(), NonZeroU32> {
    com_arm9(9, &[user_type, pointer as u32])
}

pub unsafe fn arm9_init_sdmmc(drive: u8) -> Result<(), NonZeroU32> {
    com_arm9(8, &[drive as u32])
}
pub unsafe fn arm9_check_sdmmc(drive: u8) -> Result<(), NonZeroU32> {
    com_arm9(11, &[drive as u32])
}

pub struct StorageSector([u32; 128]);
impl StorageSector {
    pub const ZEROD: Self = Self([0; _]);
}
impl Default for StorageSector {
    fn default() -> Self {
        Self::ZEROD
    }
}
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

pub unsafe fn flush_mmc() {
    #[cfg(target_arch = "arm")]
    core::arch::asm!(
        "MCR p15, 0, r0, c7, c10, 4", //drain write buffer
        in("r0") 0,
    );
    for i in 0..4 {
        for j in 0..0x20 {
            let arg = (i << 30) | (j << 5);
            #[cfg(target_arch = "arm")]
            core::arch::asm!(
                "MCR p15, 0, r0, c7, c10, 2", //clean dcache entry
                in("r0") arg,
            );
        }
    }
    #[cfg(target_arch = "arm")]
    core::arch::asm!(
        "MCR p15, 0, r0, c7, c10, 4", //drain write buffer
        "MCR p15, 0, r0, c7, c5, 0", //Flush ICache
        "MCR p15, 0, r0, c7, c6, 0", //Flush DCache
        in("r0") 0,
    );
}

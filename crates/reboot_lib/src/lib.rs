// SPDX-FileCopyrightText: 2026 Viktor Karlsson <viktor@koda.re>
// SPDX-License-Identifier: MIT

//! # Reboot-lib
//!
//! This crate contains code for writing to the hardware on DS/DSi consoles.
//! Similar to those found in the popular libnds and other associated libraries.
//! Everything is also written with explicitly these targets in mind, (i.e armv4t and armv5te)
//! such as that various assertions will likely fail if compiled for any other target.
//!
//! In order to use the crate correctly and gain access to the various functions related to the hardware, one must enable the features they wish to use:
//! arm9 - hardware used by the armv5te core in the DS/DSi
//! arm7 - hardware used by the armv4t core in the DS/DSi
//! arm9i - hardware used by the armv5te core in the DSi exclusively
//! arm7i - hardware used by the armv4t core in the DSi exclusively
//!
//! Other extra features are provided to support auxiliary functions:
//! standard_arm7 - expose a main function for a standardized arm7 binary that can be interacted with from the arm9.
//! init_nand_aes - re-initialize the AES hardware found in the DSi for NAND access, in the vast majority of cases (even for astronaut) this is not needed.
//! fatfs - bundle bindings to elm-chan's fatfs library in order to interact with filesystems.
//!

#![no_std]
#![feature(allocator_api)]
#![feature(ptr_metadata)]
#![allow(unused)]

extern crate alloc;

/// Assert a constant item at compile time, used to e.g validate the size of various system related structs.
#[macro_export]
macro_rules! const_assert {
    ($($tt:tt)*) => {
        const _: () = assert!($($tt)*);
    }
}

pub use bytemuck;
pub use mmc::*;
pub use volatile_register;

/// Structs for handling Autoboot parameters
///
/// [related GBATEK page](https://problemkaputt.de/gbatek-dsi-autoload-on-warmboot.htm)
pub mod autoboot_info;

/// DMA hardware functions and structs
///
/// [related GBATEK page 1](https://problemkaputt.de/gbatek-ds-dma-transfers.htm)
/// [related GBATEK page 2](https://problemkaputt.de/gbatek-gba-dma-transfers.htm)
pub mod dma;

mod aes;
mod allocator;
/// Functions to interact with devices on the I2C bus of the DSi
///
/// [related GBATEK page](https://problemkaputt.de/gbatek-dsi-i2c-bus.htm)
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
#[cfg(all(feature = "arm7i", feature = "standard_arm7"))]
pub mod standard_arm7;
mod swi;
pub mod timers;
mod video;
pub use bitflags;
pub mod rtc;
use core::num::NonZeroU32;

#[cfg(feature = "arm7i")]
pub mod twl_wifi;

pub use aes::*;
pub use allocator::ALLOCATOR;
pub use dma::*;
pub use interupts::*;
#[cfg(any(feature = "arm7", feature = "arm9"))]
pub use ipc::IPC_FIFO_HARDWARE;
pub use memory::VRAMCtrl;
pub use mmc::driver::*;
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

pub unsafe fn critical_function<F: FnOnce()>(closure: F) {
    let mut ime = INTERRUPT_HARDWARE.master.read();
    INTERRUPT_HARDWARE.master.write(0);
    closure();
    INTERRUPT_HARDWARE.master.write(ime);
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
#[cfg(feature = "fatfs")]
pub use fatfs_embedded;
#[allow(static_mut_refs)]
unsafe fn watchdog_trigger() {
    panic!("Watchdog triggered, code: {WD_CODE}");
}
static mut WD_CODE: u8 = 0;
use crate::timers::{Timer, TimerControl, TIMERS};
unsafe fn start_watchdog() {
    TIMERS[3].write(Timer::new(0, TimerControl::empty()));
    interupts::set_interrupt_function(Interrupt::Timer3, watchdog_trigger);
    interupts::enable_interrupt(Interrupt::Timer3);
}
unsafe fn kick_watchdog(reason: u8) {
    TIMERS[3].write(Timer::new(
        0x1,
        TimerControl::START | TimerControl::PRESCALE_1024 | TimerControl::ENABLE_IRQ,
    ));
    WD_CODE = reason;
}
unsafe fn stop_watchdog() {
    TIMERS[3].write(Timer::new(0, TimerControl::empty()));
    interupts::disable_interrupt(Interrupt::Timer3);
    WD_CODE = 255;
}
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
enum TransactionCode {
    ReadControls = Self::READ_CONTROLLER as u8,
    SetBuffer = Self::SET_BUFFER as u8,
    ReadNandEncrypted = Self::READ_NAND_ENCRYPTED as u8,
    ReadNandRaw = Self::READ_NAND_RAW as u8,
    ReadSD = Self::READ_SD as u8,
    WriteSD = Self::WRITE_SD as u8,
    WriteNand = Self::WRITE_NAND as u8,
    Boot = Self::BOOT as u8,
    DoModcrypt = Self::DO_MODCRYPT as u8,
    GenericSend = Self::GENERIC_SEND as u8,
    InitSDMMCDevice = Self::INIT_SDMMCDEVICE as u8,
    CheckSDMMCDevice = Self::CHECK_SDMMCDEVICE as u8,
    InitWifi = Self::INIT_WIFI as u8,
    SetWarmboot = Self::SET_WARMBOOT as u8,
    SetSoundChannel = Self::SET_SOUND_CHANNEL as u8,
}
impl TransactionCode {
    pub const READ_CONTROLLER: u32 = 1;
    pub const SET_BUFFER: u32 = 2;
    pub const READ_NAND_ENCRYPTED: u32 = 3;
    pub const READ_NAND_RAW: u32 = 4;
    pub const READ_SD: u32 = 5;
    pub const WRITE_SD: u32 = 10;
    pub const WRITE_NAND: u32 = 15;
    pub const BOOT: u32 = 6;
    pub const DO_MODCRYPT: u32 = 12;
    pub const GENERIC_SEND: u32 = 9;
    pub const INIT_SDMMCDEVICE: u32 = 8;
    pub const CHECK_SDMMCDEVICE: u32 = 11;
    pub const INIT_WIFI: u32 = 13;
    pub const SET_SOUND_CHANNEL: u32 = 14;
    pub const SET_WARMBOOT: u32 = 16;
}
#[cfg(feature = "standard_arm7")]
unsafe fn com_arm9(opcode: TransactionCode, data_out: &[u32]) -> Result<(), NonZeroU32> {
    //start_watchdog();
    //kick_watchdog(opcode);
    crate::critical_function(|| {
        IPC_FIFO_HARDWARE.send_raw_blocking(opcode as u8 as u32);
        for data in data_out.into_iter().copied() {
            IPC_FIFO_HARDWARE.send_raw_blocking(data);
        }
    });
    loop {
        let mut value = Err(ipc::RecieveFifoError::QueueEmpty);
        critical_function(|| value = IPC_FIFO_HARDWARE.recieve_value_raw());
        if let Ok(value) = value {
            critical_function(|| assert!(IPC_FIFO_HARDWARE.recieve_value_raw().is_err()));
            //stop_watchdog();
            match NonZeroU32::new(value) {
                Some(value) => return Err(value),
                None => return Ok(()),
            }
        } else if IPC_FIFO_HARDWARE.read_status() == 7 {
            panic!("ARM7 crashed during command {opcode:?}");
        }
    }
}

#[cfg(feature = "standard_arm7")]
pub unsafe fn arm9_send_controller_read() -> (Buttons, u8, u8) {
    let value = com_arm9(TransactionCode::ReadControls, &[])
        .map_err(|i| u32::from(i))
        .err()
        .unwrap_or(0);
    (
        Buttons::from_bits_retain(value as u16),
        (value >> 16) as u8,
        (value >> 24) as u8,
    )
}

#[cfg(feature = "standard_arm7")]
pub unsafe fn arm9_set_buffer(slice: *mut [StorageSector]) -> Result<(), NonZeroU32> {
    let (ptr, len) = slice.to_raw_parts();
    com_arm9(TransactionCode::SetBuffer, &[ptr as u32, len as u32])
}

#[cfg(feature = "standard_arm7")]
pub unsafe fn arm9_read_nand_sector_encrypted(start_sector: u32) -> Result<(), NonZeroU32> {
    com_arm9(TransactionCode::ReadNandEncrypted, &[start_sector])
}

#[cfg(feature = "standard_arm7")]
pub unsafe fn arm9_read_nand_sector_unencrypted(start_sector: u32) -> Result<(), NonZeroU32> {
    com_arm9(TransactionCode::ReadNandRaw, &[start_sector])
}

#[cfg(feature = "standard_arm7")]
pub unsafe fn arm9_read_sd_sector(start_sector: u32) -> Result<(), NonZeroU32> {
    com_arm9(TransactionCode::ReadSD, &[start_sector])
}

#[cfg(feature = "standard_arm7")]
pub unsafe fn arm9_write_sd_sector(start_sector: u32) -> Result<(), NonZeroU32> {
    com_arm9(TransactionCode::WriteSD, &[start_sector])
}

#[cfg(feature = "standard_arm7")]
pub unsafe fn arm9_write_nand_sector(start_sector: u32) -> Result<(), NonZeroU32> {
    com_arm9(TransactionCode::WriteNand, &[start_sector])
}

#[cfg(feature = "standard_arm7")]
pub unsafe fn arm9_send_arm7_boot() -> Result<(), NonZeroU32> {
    com_arm9(TransactionCode::Boot, &[])
}

#[cfg(feature = "standard_arm7")]
pub unsafe fn arm9_decrypt_modcrypt() -> Result<(), NonZeroU32> {
    com_arm9(TransactionCode::DoModcrypt, &[])
}

#[cfg(feature = "standard_arm7")]
pub unsafe fn arm9_send_arm7(user_type: u32, pointer: *mut ()) -> Result<(), NonZeroU32> {
    com_arm9(TransactionCode::GenericSend, &[user_type, pointer as u32])
}

#[cfg(feature = "standard_arm7")]
pub unsafe fn arm9_init_sdmmc(drive: u8) -> Result<(), NonZeroU32> {
    com_arm9(TransactionCode::InitSDMMCDevice, &[drive as u32])
}

#[cfg(feature = "standard_arm7")]
pub unsafe fn arm9_check_sdmmc(drive: u8) -> Result<(), NonZeroU32> {
    com_arm9(TransactionCode::CheckSDMMCDevice, &[drive as u32])
}

#[cfg(feature = "standard_arm7")]
pub unsafe fn arm9_init_nwifi(firmware_file: &mut [u8]) -> Result<(), NonZeroU32> {
    let ptr = firmware_file.as_mut_ptr();
    let len = firmware_file.len();
    com_arm9(TransactionCode::InitWifi, &[ptr as u32, len as u32])
}

#[cfg(feature = "standard_arm7")]
pub unsafe fn arm9_set_warmboot() -> Result<(), NonZeroU32> {
    com_arm9(TransactionCode::SetWarmboot, &[])
}

#[cfg(feature = "standard_arm7")]
pub unsafe fn arm9_manual_sound_write(
    buffer: &mut [u8],
    channel: u8,
    timer: u16,
    control: sound::SoundControl,
    loop_start: u16,
) -> Result<(), NonZeroU32> {
    let ptr = buffer.as_mut_ptr();
    let len = ((buffer.len() as u32) << 4) | channel as u32;
    let timer = timer as u32 | ((loop_start as u32) << 16);
    com_arm9(
        TransactionCode::SetSoundChannel,
        &[ptr as u32, len as u32, timer, control.bits()],
    )
}
unsafe impl bytemuck::NoUninit for StorageSector {}
#[derive(Clone, Copy)]
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
#[cfg(not(target_arch = "arm"))]
pub unsafe fn flush_mmc() {
    panic!()
}

#[cfg(target_arch = "arm")]
#[instruction_set(arm::a32)]
pub unsafe fn flush_mmc() {
    core::arch::asm!(
        "MCR p15, 0, r0, c7, c10, 4", //drain write buffer
        in("r0") 0,
        lateout("r0") _,
    );
    for i in 0..4 {
        for j in 0..0x20 {
            let mut arg = (i << 30) | (j << 5);
            core::arch::asm!(
                "MCR p15, 0, r0, c7, c10, 2", //clean dcache entry
                inout("r0") arg,
            );
        }
    }
    core::arch::asm!(
        "MCR p15, 0, r0, c7, c10, 4", //drain write buffer
        "MCR p15, 0, r0, c7, c5, 0", //Flush ICache
        "MCR p15, 0, r0, c7, c6, 0", //Flush DCache
        in("r0") 0,
        lateout("r0") _,
    );
}

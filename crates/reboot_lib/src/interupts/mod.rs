// SPDX-FileCopyrightText: 2026 Viktor Karlsson <viktor@koda.re>
// SPDX-License-Identifier: MIT

mod arm7;
mod arm9;
use crate::MemoryWrapper;
pub use arm7::*;
use volatile_register::*;
/// Memory wrapper around the interrupt hardware registers
pub const INTERRUPT_HARDWARE: MemoryWrapper<InteruptRegisters> =
    MemoryWrapper(0x4000208 as *mut InteruptRegisters);

/// Memory layout of interrupt hardware registers
#[repr(C)]
pub struct InteruptRegisters {
    /// Master interrupt enable register
    pub master: RW<u32>,
    _unused: u32,
    
    /// Interrupt enable (flags) register
    pub enable: RW<u32>,
    
    /// Interrupt request (flags) register
    pub request: RW<u32>,
    
    /// Auxiliary interrupt enable (flags) register
    #[cfg(feature = "arm7i")]
    pub enable2: RW<u32>,

    /// Auxiliary interrupt request (flags) register
    #[cfg(feature = "arm7i")]
    pub request2: RW<u32>,
}

static mut INTERRUPT_TABLE: [*mut fn(); 32] = [core::ptr::null_mut(); 32];
#[cfg(feature = "arm7i")]
static mut INTERRUPT_TABLE_AUX: [*mut fn(); 15] = [core::ptr::null_mut(); 15];

const AUX_INTERRUPT: u8 = 32;
const INTERRUPT_INDEX_MASK: u8 = (AUX_INTERRUPT - 1);

/// Interrupt selector
#[repr(u8)]
pub enum Interrupt {
    /// Fire on VBlank
    VBlank = 0,
    /// Fire on HBlank
    HBlank = 1,
    /// Fire when VCOUNT register matches VCOUNT counter.
    VCounterMatch = 2,
    /// Fire when timer 0 runs out
    Timer0 = 3,
    /// Fire when timer 1 runs out
    Timer1 = 4,
    /// Fire when timer 2 runs out
    Timer2 = 5,
    /// Fire when timer 3 runs out
    Timer3 = 6,

    /// Fire from RTC action
    #[cfg(feature = "arm7")]
    RTC = 7,

    DMA0 = 8,
    DMA1 = 9,
    DMA2 = 10,
    DMA3 = 11,

    /// Fire when controller buttons are pressed
    Keypad = 12,
    
    Slot2 = 13,
    /// Fire on IPC sync signal
    IPCSync = 16,
    /// Fire when IPC FIFO empty
    IPCEmpty = 17,
    /// Fire when IPC FIFO nonempty
    IPCNonEmpty = 18,
    
    Slot1TransferComplete = 19,
    Slot1IREQMC = 20,

    /// Fire on hinge opening
    #[cfg(feature = "arm7")]
    HingeOpen = 22,

    /// Fire on SPI actions
    #[cfg(feature = "arm7")]
    SPI = 23,

    /// Fire on WiFi actions
    #[cfg(feature = "arm7")]
    Wifi = 24,

    NDMA0 = 28,
    NDMA1 = 29,
    NDMA2 = 30,
    NDMA3 = 31,

    #[cfg(feature = "arm7i")]
    GPIO180 = 0 + AUX_INTERRUPT,
    #[cfg(feature = "arm7i")]
    GPIO181 = 1 + AUX_INTERRUPT,
    #[cfg(feature = "arm7i")]
    GPIO182 = 2 + AUX_INTERRUPT,

    /// Fire on Headphone connection
    #[cfg(feature = "arm7i")]
    HeadphoneConnect = 5 + AUX_INTERRUPT,

    /// Fire on power button press
    #[cfg(feature = "arm7i")]
    Powerbutton = 6 + AUX_INTERRUPT,

    /// Fire on sound enable
    #[cfg(feature = "arm7i")]
    SoundEnableOutput = 7 + AUX_INTERRUPT,

    /// Fire on SDMMC status change
    #[cfg(feature = "arm7i")]
    SDMMC = 8 + AUX_INTERRUPT,

    /// Fire on SDMMC data1 change
    #[cfg(feature = "arm7i")]
    SDMMCData1 = 9 + AUX_INTERRUPT,

    /// Fire on SDIO status change
    #[cfg(feature = "arm7i")]
    SDIO = 10 + AUX_INTERRUPT,

    /// Fire on SDIO data1 change
    #[cfg(feature = "arm7i")]
    SDIOData1 = 11 + AUX_INTERRUPT,

    /// Fire on AES engine actions
    #[cfg(feature = "arm7i")]
    AES = 12 + AUX_INTERRUPT,

    /// Fire on I2C actions
    #[cfg(feature = "arm7i")]
    I2C = 13 + AUX_INTERRUPT,

    /// Fire on microphone action
    #[cfg(feature = "arm7i")]
    MicrophoneExt = 14 + AUX_INTERRUPT,
}
#[cfg(all(feature = "arm7", not(feature = "arm9")))]
pub use arm7::init_interrupts;
#[cfg(all(feature = "arm9", not(feature = "arm7")))]
pub use arm9::init_interrupts;

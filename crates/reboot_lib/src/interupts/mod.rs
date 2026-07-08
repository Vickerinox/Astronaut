mod arm7;
mod arm9;
use crate::MemoryWrapper;
pub use arm7::*;
use volatile_register::*;
pub const INTERUPT_HARDWARE: MemoryWrapper<InteruptRegisters> =
    MemoryWrapper(0x4000208 as *mut InteruptRegisters);

#[repr(C)]
pub struct InteruptRegisters {
    pub master: RW<u32>,
    _unused: u32,
    pub enable: RW<u32>,
    pub request: RW<u32>,
    pub enable2: RW<u32>,
    pub request2: RW<u32>,
}


static mut INTERRUPT_TABLE: [*mut fn(); 32] = [core::ptr::null_mut(); 32];
#[cfg(feature = "arm7")]
static mut INTERRUPT_TABLE_AUX: [*mut fn(); 15] = [core::ptr::null_mut(); 15];

const AUX_INTERRUPT: u8 = 32;
const INTERRUPT_INDEX_MASK: u8 = (AUX_INTERRUPT - 1);

#[repr(u8)]
pub enum Interrupt {
    VBlank = 0,
    HBlank = 1,
    VCounterMatch = 2,
    Timer0 = 3,
    Timer1 = 4,
    Timer2 = 5,
    Timer3 = 6,
    
    #[cfg(feature = "arm7")]
    RTC = 7,

    DMA0 = 8,
    DMA1 = 9,
    DMA2 = 10,
    DMA3 = 11,
    Keypad = 12,
    Slot2 = 13,
    IPCSync = 16,
    IPCEmpty = 17,
    IPCNonEmpty = 18,
    Slot1TransferComplete = 19,
    Slot1IREQMC = 20,

    #[cfg(feature = "arm7")]
    HingeOpen = 22,
    #[cfg(feature = "arm7")]
    SPI = 23,
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


    #[cfg(feature = "arm7i")]
    HeadphoneConnect = 5 + AUX_INTERRUPT,
    
    #[cfg(feature = "arm7i")]
    Powerbutton = 6 + AUX_INTERRUPT,
    
    #[cfg(feature = "arm7i")]
    SoundEnableOutput = 7 + AUX_INTERRUPT,
    
    #[cfg(feature = "arm7i")]
    SDMMC = 8 + AUX_INTERRUPT,

    #[cfg(feature = "arm7i")]
    SDMMCData1 = 9 + AUX_INTERRUPT,

    #[cfg(feature = "arm7i")]
    SDIO = 10 + AUX_INTERRUPT,

    #[cfg(feature = "arm7i")]
    SDIOData1 = 11 + AUX_INTERRUPT,

    #[cfg(feature = "arm7i")]
    AES = 12 + AUX_INTERRUPT,

    #[cfg(feature = "arm7i")]
    I2C = 13 + AUX_INTERRUPT,

    #[cfg(feature = "arm7i")]
    MicrophoneExt = 14 + AUX_INTERRUPT,
}
#[cfg(all(feature = "arm7", not(feature = "arm9")))]
pub use arm7::init_interrupts;
#[cfg(all(feature = "arm9", not(feature = "arm7")))]
pub use arm9::init_interrupts;
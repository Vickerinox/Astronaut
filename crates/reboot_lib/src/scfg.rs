// SPDX-FileCopyrightText: 2026 Viktor Karlsson <viktor@koda.re>
// SPDX-License-Identifier: MIT


#[cfg(any(feature = "arm9i", feature = "arm7i"))]
pub const SCFG_HARDWARE: MemoryWrapper<SCFGHardware> = MemoryWrapper(0x4004000 as *mut SCFGHardware);

use crate::MemoryWrapper;

bitflags::bitflags! {

    #[derive(Clone, Copy)]
    pub struct ROMSCFG: u32 {

        #[cfg(feature = "arm9i")]
        const ARM9_UPPER_BIOS_HALF = (1<<0);

        #[cfg(feature = "arm9i")]
        const ARM9_NDS_MODE_BIOS = (1<<1);


        #[cfg(feature = "arm7i")]
        const ARM7_UPPER_BIOS_HALF = (1<<8);

        #[cfg(feature = "arm7i")]
        const ARM7_NDS_MODE_BIOS = (1<<9);

        #[cfg(feature = "arm7i")]
        const CONSOLE_ID_ACCESS = (1<<10);
    }

    #[derive(Clone, Copy)]
    pub struct ClockSCFG: u16 {

        #[cfg(feature = "arm9i")]
        const ARM9_TWL_SPEED = (1<<0);

        #[cfg(feature = "arm9i")]
        const DSP_CLOCK = (1<<1);

        #[cfg(feature = "arm9i")]
        const CAMERA_CLOCK = (1<<2);

        #[cfg(feature = "arm9i")]
        const CAMERA_EXTERNAL_CLOCK = (1<<8);

        #[cfg(feature = "arm7i")]
        const SDMMC_CLOCK = (1<<0);

        #[cfg(feature = "arm7i")]
        const LCD_CLOCK = (1<<1);

        #[cfg(feature = "arm7i")]
        const UNKNOWN_CLOCK = (1<<2);

        #[cfg(feature = "arm7i")]
        const NWRAM_CLOCK = (1<<7);

        #[cfg(feature = "arm7i")]
        const TSC_CLOCK = (1<<8);
    }

    #[derive(Clone, Copy)]
    pub struct ResetSCFG: u16 {

        #[cfg(feature = "arm7i")]
        const ARM7_SEL = (1<<0);

        #[cfg(feature = "arm7i")]
        const CPU_JTAG = (1<<1);

        #[cfg(feature = "arm7i")]
        const DSP_JTAG = (1<<8);
    }


    #[derive(Clone, Copy)]
    pub struct ExtSCFG: u32 {

        #[cfg(any(feature = "arm9i", feature = "arm7i"))]
        const NEW_DMA_ENABLE = (1<<0);

        #[cfg(feature = "arm7i")]
        const NEW_SOUND_DMA_ENABLE = (1<<1);
        #[cfg(feature = "arm7i")]
        const NEW_SOUND_ENABLE = (1<<2);

        #[cfg(any(feature = "arm9i", feature = "arm7i"))]
        const NEW_CART_CIRCUIT_ENABLE = (1<<7);


        #[cfg(any(feature = "arm9i", feature = "arm7i"))]
        const NEW_INTERRUPTS_ENABLE = (1<<8);
        #[cfg(feature = "arm7i")]
        const NEW_SPI_CLOCK_ENABLE = (1<<9);
        #[cfg(feature = "arm7i")]
        const EXTENDED_SOUND_DMA_ENABLE = (1<<10);

        #[cfg(feature = "arm7i")]
        const EXTENDED_UNKNOWN_2 = (1<<11);

        #[cfg(any(feature = "arm9i", feature = "arm7i"))]
        const EXTENDED_LCD_CIRCUIT_ENABLE = (1<<12);

        #[cfg(any(feature = "arm9i", feature = "arm7i"))]
        const MAIN_MEM_LIMIT_4MB = (1<<14);
        #[cfg(any(feature = "arm9i", feature = "arm7i"))]
        const MAIN_MEM_LIMIT_16MB = (2<<14);
        #[cfg(any(feature = "arm9i", feature = "arm7i"))]
        const MAIN_MEM_LIMIT_32MB = (3<<14);

        #[cfg(any(feature = "arm9i", feature = "arm7i"))]
        const ACCESS_NEW_VRAM = (1<<13);
        #[cfg(any(feature = "arm9i", feature = "arm7i"))]
        const ACCESS_NEW_DMA = (1<<16);

        #[cfg(feature = "arm9i")]
        const ACCESS_CAMERA = (1<<17);
        #[cfg(feature = "arm9i")]
        const ACCESS_DSP = (1<<18);

        #[cfg(feature = "arm7i")]
        const ACCESS_AES = (1<<17);
        #[cfg(feature = "arm7i")]
        const ACCESS_SDMMC = (1<<18);
        #[cfg(feature = "arm7i")]
        const ACCESS_SDIO = (1<<19);
        #[cfg(feature = "arm7i")]
        const ACCESS_NEW_MICROPHONE = (1<<20);
        #[cfg(feature = "arm7i")]
        const ACCESS_NEW_SOUND = (1<<21);
        #[cfg(feature = "arm7i")]
        const ACCESS_I2C = (1<<22);
        #[cfg(feature = "arm7i")]
        const ACCESS_GPIO = (1<<23);


        #[cfg(any(feature = "arm9i", feature = "arm7i"))]
        const ACCESS_CART_SLOT2 = (1<<24);

        #[cfg(any(feature = "arm9i", feature = "arm7i"))]
        const ACCESS_NWRAM = (1<<25);

        #[cfg(feature = "arm7i")]
        const ACCESS_UNKNOWN = (1<<28);

        #[cfg(any(feature = "arm9i", feature = "arm7i"))]
        const ACCESS_SCFG = (1<<31);
    }
}

#[cfg(all(feature = "arm9i", not(feature = "arm7i")))]
const FIRM_ACCESS_ARM9I: u32 = ExtSCFG::all().bits()
    ^ ExtSCFG::NEW_DMA_ENABLE.bits()
    ^ ExtSCFG::NEW_CART_CIRCUIT_ENABLE.bits();

#[cfg(all(feature = "arm9i", not(feature = "arm7i")))]
crate::const_assert!(
    FIRM_ACCESS_ARM9I == 0x8307F100,
    "Invalid Definition of ExtSCFG"
);

#[cfg(all(feature = "arm7i", not(feature = "arm9i")))]
const FIRM_ACCESS_ARM7I: u32 = ExtSCFG::all().bits()
    ^ ExtSCFG::NEW_DMA_ENABLE.bits()
    ^ ExtSCFG::NEW_CART_CIRCUIT_ENABLE.bits()
    ^ ExtSCFG::EXTENDED_SOUND_DMA_ENABLE.bits();

#[cfg(all(feature = "arm7i", not(feature = "arm9i")))]
crate::const_assert!(
    FIRM_ACCESS_ARM7I == 0x93FFFB06,
    "Invalid Definition of ExtSCFG"
);

#[repr(C)]
pub struct SCFGHardware {
    roms: ROMSCFG,
    clock: ClockSCFG,
    reset: ResetSCFG,
    features: ExtSCFG,
}

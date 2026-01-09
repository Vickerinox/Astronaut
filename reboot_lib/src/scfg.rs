
bitflags::bitflags! {
    #[derive(Clone, Copy)]
    pub struct ROMSCFG: u16 {
        const ARM9_UPPER_BIOS_HALF = (1<<0);
        const ARM9_NDS_MODE_BIOS = (1<<1);
        
        const ARM7_UPPER_BIOS_HALF = (1<<8);
        const ARM7_NDS_MODE_BIOS = (1<<9);
        const CONSOLE_ID_ACCESS = (1<<10);
    }

    #[derive(Clone, Copy)]
    pub struct ClockSCFG: u16 {
        const SDMMC_CLOCK = (1<<0);
        const LCD_CLOCK = (1<<1);
        const UNKNOWN_CLOCK = (1<<2);
        const NWRAM_CLOCK = (1<<7);
        const TSC_CLOCK = (1<<8);
    }

    #[derive(Clone, Copy)]
    pub struct ResetSCFG: u16 {
        const ARM7_SEL = (1<<0);
        const CPU_JTAG = (1<<1);
        const DSP_JTAG = (1<<8);
    }


    #[derive(Clone, Copy)]
    pub struct ExtSCFG: u32 {
        const NEW_DMA_ENABLE = (1<<0);
        const NEW_SOUND_DMA_ENABLE = (1<<1);
        const NEW_SOUND_ENABLE = (1<<2);
        const NEW_CART_CIRCUIT_ENABLE = (1<<7);
        const NEW_ARM7_INTERRUPTS_ENABLE = (1<<8);
        const NEW_SPI_CLOCK_ENABLE = (1<<9);
        const EXTENDED_SOUND_DMA_ENABLE = (1<<10);
        const EXTENDED_LCD_CIRCUIT_ENABLE = (1<<12);

        const MAIN_MEM_LIMIT_4MB = (1<<14);
        const MAIN_MEM_LIMIT_16MB = (2<<14);
        const MAIN_MEM_LIMIT_32MB = (3<<14);

        const ACCESS_NEW_VRAM = (1<<13);
        const ACCESS_NEW_DMA = (1<<16);
        const ACCESS_AES = (1<<17);
        const ACCESS_SDMMC = (1<<18);
        const ACCESS_SDIO = (1<<19);
        const ACCESS_NEW_MICROPHONE = (1<<20);
        const ACCESS_NEW_SOUND = (1<<21);
        const ACCESS_I2C = (1<<22);
        const ACCESS_GPIO = (1<<23);
        const ACCESS_NWRAM = (1<<25);
        const ACCESS_UNKNWON = (1<<28);
        const ACCESS_SCFG = (1<<31);
        
    }

}


#[repr(C)]
pub struct SCFGHardware {

}
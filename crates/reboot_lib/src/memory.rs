use volatile_register::WO;

use crate::MemoryWrapper;

pub const RAM_START: usize = 0x2_000_000;
pub const RAM_SIZE: usize = 0xFF0_000;
pub const SHARED_WRAM_START: usize = 0x3000000;
pub const SHARED_WRAM_SIZE: usize = 0x8000;

pub const VRAM_BANK_A_LCDC: usize = 0x0680_0000;
pub const VRAM_BANK_B_LCDC: usize = 0x0682_0000;
pub const VRAM_BANK_C_LCDC: usize = 0x0684_0000;
pub const VRAM_BANK_D_LCDC: usize = 0x0686_0000;
pub const VRAM_BANK_E_LCDC: usize = 0x0688_0000;
pub const VRAM_BANK_F_LCDC: usize = 0x0689_0000;
pub const VRAM_BANK_G_LCDC: usize = 0x0689_4000;
pub const VRAM_BANK_H_LCDC: usize = 0x0689_8000;
pub const VRAM_BANK_I_LCDC: usize = 0x068A_0000;

pub const VRAM_BANK_A_SIZE: usize = 1024 * 128;
pub const VRAM_BANK_B_SIZE: usize = 1024 * 128;
pub const VRAM_BANK_C_SIZE: usize = 1024 * 128;
pub const VRAM_BANK_D_SIZE: usize = 1024 * 128;
pub const VRAM_BANK_E_SIZE: usize = 1024 * 64;
pub const VRAM_BANK_F_SIZE: usize = 1024 * 16;
pub const VRAM_BANK_G_SIZE: usize = 1024 * 16;
pub const VRAM_BANK_H_SIZE: usize = 1024 * 32;
pub const VRAM_BANK_I_SIZE: usize = 1024 * 16;


bitflags::bitflags! {
    #[derive(Clone, Copy)]
    pub struct VRAMCtrl: u8 {
        // Enable this VRAM bank
        const ENABLE = (1<<7);
        
        // Set the offset of this bank to 0 within the MST
        const OFFSET_0 = (0<<3);
        // Set the offset of this bank to 1 within the MST
        const OFFSET_1 = (1<<3);
        // Set the offset of this bank to 2 within the MST
        const OFFSET_2 = (2<<3);
        // Set the offset of this bank to 3 within the MST
        const OFFSET_3 = (3<<3);
        
        // LCD Mapping for the VRAM bank
        const MST_0 = 0;
        const MST_1 = 1;
        const MST_2 = 2;
        const MST_3 = 3;
        const MST_4 = 4;
        const MST_5 = 5;
        const MST_6 = 6;
        const MST_7 = 7;

        const LCD_MAPPED = 0;
    }

}
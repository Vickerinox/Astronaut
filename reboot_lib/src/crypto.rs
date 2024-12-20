use crate::RegisterWrapper;
use volatile_register::*;

pub const AES_HARDWARE: RegisterWrapper<AESEngine> = RegisterWrapper(0x4004400 as *mut AESEngine);

pub const NAND_KEY_Y: [u8; 16] = 0xFFFEFB4E_29590258_2A680F5F_1A4F3E79u128.to_le_bytes();

#[repr(C)]
pub struct AESEngine {
    pub master_control: RW<u32>,
    pub extra_blocks: WO<u16>,
    pub payload_blocks: WO<u16>,
    pub write_fifo: WO<u32>,
    pub read_fifo: RO<u32>,
    _padding: [u32; 4],
    pub iv: [WO<u32>; 4],
    pub mac: [WO<u32>; 4],
    pub keyslots: [KeySlot; 4],
}

#[repr(C)]
pub struct KeySlot {
    pub key_n: [WO<u32>; 4],
    pub key_x: [WO<u32>; 4],
    pub key_y: [WO<u32>; 4],
}

impl AESEngine {
    pub fn load_keys(slot: usize, key_x: &[u8], key_y: &[u8]) {}
    pub unsafe fn reset(&self) {
        self.master_control.write((1 << 10) | (1 << 11));
        self.master_control.write((1 << 10) | (1 << 11));
    }
    pub unsafe fn wait_aes_busy(&self) {
        while self.master_control.read() & (1 << 31) > 0 {}
    }
    pub unsafe fn mmc_read_decrypt(&self, data: &mut [u32], ctr_base: &[u32; 4], sector: u32) -> Result<(), ()> {
        use crate::ndma::{Control, NDMA_HARDWARE};
        self.master_control.write(0);
        self.reset();
        let length = (data.len() << 2) as u32;
        self.load_iv(&ctr_base);
        self.set_block_count((length >> 4) as u16);
        let in_dma = crate::ndma::ChannelConfig {
            word_count: length >> 2,
            block_size: 4,
            timing: 8,
            fill_mode: 0,
            control: Control::DST_MODE_FIXED
                | Control::SRC_MODE_FIXED
                | Control::BLOCK_SIZE_4
                | Control::START_ARM7_WRITE_AES
                | Control::ENABLE,
        };
        NDMA_HARDWARE.set_raw_dma(1, in_dma, 0x400490C as _, 0x4004408 as _);
        let out_dma = crate::ndma::ChannelConfig {
            word_count: length >> 2,
            block_size: 4,
            timing: 8,
            fill_mode: 0,
            control: Control::SRC_MODE_FIXED
                | Control::DST_MODE_INCREMENT
                | Control::BLOCK_SIZE_4
                | Control::START_ARM7_READ_AES
                | Control::ENABLE,
        };
        NDMA_HARDWARE.set_raw_dma(0, out_dma, 0x400440C as _, data as *mut [u32] as _);
        self.start((0 << 14) | (3 << 12) | (2 << 28));
        let a = crate::read_sectors(crate::DeviceSelect::EMMC, sector, core::slice::from_raw_parts_mut(core::ptr::null_mut(), length as usize));
        NDMA_HARDWARE.await_channel(0);
        NDMA_HARDWARE.await_channel(1);
        self.wait_aes_busy();
        match a {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
    }
    //crypt a block of data in place
    pub unsafe fn ctr_crypt_block(&self, data: &mut [u32], ctr: &[u32; 4]) {
        use crate::ndma::{Control, NDMA_HARDWARE};
        self.master_control.write(0);
        self.reset();
        let length = (data.len() << 2) as u32;
        self.load_iv(ctr);
        self.set_block_count((length >> 4) as u16);

        let in_dma = crate::ndma::ChannelConfig {
            word_count: length >> 2,
            block_size: 4,
            timing: 8,
            fill_mode: 0,
            control: Control::DST_MODE_FIXED
                | Control::SRC_MODE_INCREMENT
                | Control::BLOCK_SIZE_4
                | Control::START_ARM7_WRITE_AES
                | Control::ENABLE,
        };
        NDMA_HARDWARE.set_raw_dma(1, in_dma, data as *mut [u32] as _, 0x4004408 as _);
        let out_dma = crate::ndma::ChannelConfig {
            word_count: length >> 2,
            block_size: 4,
            timing: 8,
            fill_mode: 0,
            control: Control::SRC_MODE_FIXED
                | Control::DST_MODE_INCREMENT
                | Control::BLOCK_SIZE_4
                | Control::START_ARM7_READ_AES
                | Control::ENABLE,
        };
        NDMA_HARDWARE.set_raw_dma(0, out_dma, 0x400440C as _, data as *mut [u32] as _);
        self.start((0 << 14) | (3 << 12) | (2 << 28));

        NDMA_HARDWARE.await_channel(0);
        NDMA_HARDWARE.await_channel(1);
        self.wait_aes_busy();
    }
    pub unsafe fn start(&self, flags: u32) {
        self.master_control.write(flags | (1 << 31));
    }
    pub unsafe fn set_block_count(&self, count: u16) {
        self.payload_blocks.write(count);
    }
    pub unsafe fn load_iv(&self, iv: &[u32; 4]) {
        for i in 0..4 {
            self.iv[i].write(iv[i]);
        }
    }
    pub unsafe fn wait_key_busy(&self) {
        while self.master_control.read() & (1 << 25) > 0 {}
    }
    pub unsafe fn set_key_slot(&self, slot: usize) {
        self.wait_key_busy();
        let read = self.master_control.read();
        let write = (read & !(3 << 26)) | (1 << 24) | ((slot as u32) << 26);
        self.master_control.write(write);
    }
}

pub unsafe fn nand_crypt_init(keyslot: usize) {
    core::ptr::write_volatile(
        0x4004008 as *mut u32,
        core::ptr::read_volatile(0x4004008 as *const u32) | (1 << 17) | (1 << 2),
    );
    AES_HARDWARE.reset();
    AES_HARDWARE.reset();
    AES_HARDWARE.wait_key_busy();
    let keyslot = (keyslot as u32) & 3;
    AES_HARDWARE
        .master_control
        .write((2 << 28) | (keyslot << 26) | (1 << 24) | (1 << 31) | (2 << 12) | (1 << 14));
    AES_HARDWARE.set_key_slot(0);
    AES_HARDWARE.wait_key_busy();
}
pub unsafe fn load_nand_key_x(keyslot: usize) {
    AES_HARDWARE.keyslots[keyslot].key_x[0].write(core::ptr::read_volatile(0x4004D00 as *mut u32));
    AES_HARDWARE.keyslots[keyslot].key_x[1]
        .write(core::ptr::read_volatile(0x4004D00 as *mut u32) ^ 0x24EE6906);
    AES_HARDWARE.keyslots[keyslot].key_x[2]
        .write(core::ptr::read_volatile(0x4004D04 as *mut u32) ^ 0xE65B601D);
    AES_HARDWARE.keyslots[keyslot].key_x[3].write(core::ptr::read_volatile(0x4004D04 as *mut u32));
}
pub unsafe fn load_nand_key_y(keyslot: usize, key: &[u32; 4]) {
    AES_HARDWARE.keyslots[keyslot].key_y[0].write(key[0]);
    AES_HARDWARE.keyslots[keyslot].key_y[1].write(key[1]);
    AES_HARDWARE.keyslots[keyslot].key_y[2].write(key[2]);
    AES_HARDWARE.keyslots[keyslot].key_y[3].write(key[3]);
}

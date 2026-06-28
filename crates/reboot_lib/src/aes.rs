use crate::MemoryWrapper;
use common::bootstrap::TWLHeader;
use volatile_register::*;

pub const AES_HARDWARE: MemoryWrapper<AESEngine> = MemoryWrapper(0x4004400 as *mut AESEngine);

pub const NAND_KEY_Y: [u8; 16] = 0xFFFEFB4E_29590258_2A680F5F_1A4F3E79u128.to_le_bytes();

bitflags::bitflags! {
    #[derive(Clone, Copy)]
    pub struct AESCnt: u32 {
        const FLUSH_WRITE_FIFO = (1<<10);
        const FLUSH_READ_FIFO = (1<<11);
        const KEY_SELECT = (1<<24);
        const KEY_SLOT_0 = (0<<26);
        const KEY_SLOT_1 = (1<<26);
        const KEY_SLOT_2 = (2<<26);
        const KEY_SLOT_3 = (3<<26);
        const KEY_BUSY = (1<<25);
        const MODE_CCM_DEC = (0<<28);
        const MODE_CCM_ENC = (1<<28);
        const MODE_CTR = (2<<28);
        const START = (1<<31);
    }

}

#[repr(C)]
pub struct AESEngine {
    pub master_control: RW<AESCnt>,
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
impl KeySlot {
    pub unsafe fn load_key_x(&self, key: &[u32; 4]) {
        for (reg, value) in self.key_x.iter().zip(key) {
            reg.write(*value);
        }
    }
    pub unsafe fn load_key_y(&self, key: &[u32; 4]) {
        for (reg, value) in self.key_y.iter().zip(key) {
            reg.write(*value);
        }
    }
}

impl AESEngine {
    pub unsafe fn init_from_header(&self, header: &TWLHeader, console_id: [u32; 2]) {
        if header.is_dsi_mode() {
            self.master_control.write(AESCnt::empty());
            self.reset();
            self.reset();
            self.wait_key_busy();

            //TODO: support debug keys
            self.wait_key_busy();
            let module = &self.keyslots[0];
            module.key_x[0].write(0x746E694E);
            module.key_x[1].write(0x6F646E65);
            module.key_x[2].write(header.head.tid);
            module.key_x[3].write(header.head.tid.swap_bytes());
            module.key_y[0].write(header.arm9i_sha1[0]);
            module.key_y[1].write(header.arm9i_sha1[1]);
            module.key_y[2].write(header.arm9i_sha1[2]);
            module.key_y[3].write(header.arm9i_sha1[3]);

            self.wait_key_busy();
            let nand = &self.keyslots[3];
            nand.key_x[0].write(console_id[0]);
            nand.key_x[1].write(console_id[0] ^ 0x24EE6906);
            nand.key_x[2].write(console_id[1] ^ 0xE65B601D);
            nand.key_x[3].write(console_id[1]);

            self.wait_key_busy();
            nand.key_y[3].write(0xE1A00005);
            self.wait_key_busy();
        }
    }
    pub fn load_keys(slot: usize, key_x: &[u8], key_y: &[u8]) {}
    pub unsafe fn reset(&self) {
        self.master_control
            .write(AESCnt::FLUSH_WRITE_FIFO | AESCnt::FLUSH_READ_FIFO);
        self.master_control
            .write(AESCnt::FLUSH_WRITE_FIFO | AESCnt::FLUSH_READ_FIFO);
    }
    pub unsafe fn wait_aes_busy(&self) {
        while self.master_control.read().contains(AESCnt::START) {}
    }
    //crypt a block of data in place
    pub unsafe fn ctr_crypt_block(&self, data: &mut [u32], ctr: &[u32; 4]) {
        let len = data.len() as u32;
        use crate::ndma::{Control, NDMA_HARDWARE};
        self.master_control.write(AESCnt::empty());
        self.reset();
        self.load_iv(ctr);
        self.set_block_count((len >> 2) as u16);

        let in_dma = crate::ndma::ChannelConfig {
            word_count: len,
            block_size: 4,
            timing: 8,
            fill_mode: 0,
            control: Control::DST_MODE_FIXED
                | Control::SRC_MODE_INCREMENT
                | Control::BLOCK_SIZE_4
                | Control::START_ARM7_WRITE_AES
                | Control::ENABLE,
        };
        let out_dma = crate::ndma::ChannelConfig {
            word_count: len,
            block_size: 4,
            timing: 8,
            fill_mode: 0,
            control: Control::SRC_MODE_FIXED
                | Control::DST_MODE_INCREMENT
                | Control::BLOCK_SIZE_4
                | Control::START_ARM7_READ_AES
                | Control::ENABLE,
        };
        let ptr = data as *mut [u32] as *mut u32;
        NDMA_HARDWARE.set_raw_dma(0, out_dma, 0x400440C as _, ptr as _);
        NDMA_HARDWARE.set_raw_dma(1, in_dma, ptr as _, 0x4004408 as _);

        self.start((0 << 14) | (3 << 12) | (2 << 28));

        NDMA_HARDWARE.await_channel(0);
        NDMA_HARDWARE.await_channel(1);

        self.wait_aes_busy();
    }

    //crypt a block of data in place
    pub unsafe fn ctr_crypt_block_cpu(&self, data: &mut [u32], ctr: &[u32; 4]) {
        let len = data.len() as u32;
        use crate::ndma::{Control, NDMA_HARDWARE};
        self.master_control.write(AESCnt::empty());
        self.reset();
        self.load_iv(ctr);
        self.set_block_count((len >> 2) as u16);

        let in_dma = crate::ndma::ChannelConfig {
            word_count: len,
            block_size: 4,
            timing: 8,
            fill_mode: 0,
            control: Control::DST_MODE_FIXED
                | Control::SRC_MODE_INCREMENT
                | Control::BLOCK_SIZE_4
                | Control::START_ARM7_WRITE_AES
                | Control::ENABLE,
        };
        let out_dma = crate::ndma::ChannelConfig {
            word_count: len,
            block_size: 4,
            timing: 8,
            fill_mode: 0,
            control: Control::SRC_MODE_FIXED
                | Control::DST_MODE_INCREMENT
                | Control::BLOCK_SIZE_4
                | Control::START_ARM7_READ_AES
                | Control::ENABLE,
        };
        let ptr = data as *mut [u32] as *mut u32;
        //NDMA_HARDWARE.set_raw_dma(0, out_dma, 0x400440C as _, ptr as _);
        //NDMA_HARDWARE.set_raw_dma(1, in_dma, ptr as _, 0x4004408 as _);

        self.start((0 << 14) | (3 << 12) | (2 << 28));

        //NDMA_HARDWARE.await_channel(0);
        //NDMA_HARDWARE.await_channel(1);

        self.wait_aes_busy();
    }
    pub unsafe fn start(&self, flags: u32) {
        self.master_control
            .write(AESCnt::from_bits_retain(flags) | AESCnt::START);
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
        while self.master_control.read().contains(AESCnt::KEY_BUSY) {}
    }
    pub unsafe fn set_key_slot(&self, slot: usize) {
        self.wait_key_busy();
        let read = self.master_control.read().bits();
        let write = (read & !(3 << 26)) | (1 << 24) | ((slot as u32) << 26);
        self.master_control.write(AESCnt::from_bits_retain(write));
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
        .write(AESCnt::MODE_CTR | AESCnt::from_bits_retain(keyslot << 26) | AESCnt::KEY_SELECT);
    AES_HARDWARE.set_key_slot(keyslot as usize);
    AES_HARDWARE.wait_key_busy();
}
pub unsafe fn load_nand_key_x(keyslot: usize, console_id: [u32; 2]) {
    AES_HARDWARE.keyslots[keyslot].key_x[0].write(console_id[0]);
    AES_HARDWARE.keyslots[keyslot].key_x[1].write(console_id[0] ^ 0x24EE6906);
    AES_HARDWARE.keyslots[keyslot].key_x[2].write(console_id[1] ^ 0xE65B601D);
    AES_HARDWARE.keyslots[keyslot].key_x[3].write(console_id[1]);
}
pub unsafe fn load_nand_key_y(keyslot: usize, key: &[u32; 4]) {
    AES_HARDWARE.keyslots[keyslot].key_y[0].write(key[0]);
    AES_HARDWARE.keyslots[keyslot].key_y[1].write(key[1]);
    AES_HARDWARE.keyslots[keyslot].key_y[2].write(key[2]);
    AES_HARDWARE.keyslots[keyslot].key_y[3].write(key[3]);
}

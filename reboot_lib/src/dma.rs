use volatile_register::{RW, WO};

use crate::MemoryWrapper;

pub const DMA_HARDWARE: MemoryWrapper<DMAHardware> = MemoryWrapper(0x400_00B0 as *mut DMAHardware);
pub struct DMAHardware([DMAChannel; 4]);
pub struct DMAChannel {
    source_address: WO<u32>,
    destination_address: WO<u32>,
    word_count: WO<u16>,
    control: RW<DMACnt>,
}
impl DMAHardware {
    pub unsafe fn reset(&self) {
        for channel in &self.0 {
            channel.reset();
        }
    }
}
impl DMAChannel {
    pub unsafe fn reset(&self) {
        self.source_address.write(0);
        self.destination_address.write(0);
        self.word_count.write(0);
        self.control.write(DMACnt::empty());
    }
}
bitflags::bitflags! {
    #[derive(Clone, Copy)]
    pub struct DMACnt: u16 {
        const DEST_INCREMENT = (0 << 5);
        const DEST_DECREMENT = (1 << 5);
        const DEST_FIXED = (2 << 5);
        const DEST_RELOADED = (3 << 5);
        const SOURCE_INCREMENT = (0 << 7);
        const SOURCE_DECREMENT = (1 << 7);
        const SOURCE_FIXED = (2 << 7);
        const REPEAT = (1 << 9);
        const TRANSFER_FULL_WORDS = (1<<10);
        const GAME_PACK_DRQ = (1<<11);
        const START_IMMEDIATELY = (0<<12);
        const START_HBLANK = (1<<12);
        const START_VBLANK = (2<<12);
        const START_SPECIALIZED= (3<<12);
        const ENABLE_IRQ = (1<<14);
        const ENABLE = (1<<15);
    }
}

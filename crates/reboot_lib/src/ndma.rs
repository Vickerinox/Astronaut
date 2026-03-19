use crate::MemoryWrapper;
use volatile_register::*;
pub const NDMA_HARDWARE: MemoryWrapper<NDMA> = MemoryWrapper(0x4004100 as *mut NDMA);

#[repr(C)]
pub struct NDMA {
    global_control: WO<GlobalControl>,
    channels: [NDMAChannel; 4],
}
impl NDMA {
    pub unsafe fn reset(&self) {
        self.global_control.write(GlobalControl::empty());
        for channel in &self.channels {}
    }
}
impl NDMAChannel {
    pub unsafe fn reset(&self) {
        self.src.write(0);
        self.dst.write(0);
        self.word_count.write(0);
        self.block_size.write(0);
        self.timing.write(0);
        self.fill_mode.write(0);
        self.control.write(Control::empty());
    }
}
#[repr(C)]
pub struct NDMAChannel {
    src: WO<u32>,
    dst: WO<u32>,
    word_count: WO<u32>,
    block_size: WO<u32>,
    timing: WO<u32>,
    fill_mode: WO<u32>,
    control: RW<Control>,
}
#[repr(C)]
pub struct ChannelConfig {
    pub word_count: u32,
    pub block_size: u32,
    pub timing: u32,
    pub fill_mode: u32,
    pub control: Control,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct Control: u32 {
        const SRC_MODE_INCREMENT = (0 << 13);
        const SRC_MODE_DECREMENT = (1 << 13);
        const SRC_MODE_FIXED = (2 << 13);
        const SRC_MODE_FILL = (3 << 13);
        const DST_MODE_INCREMENT = (0 << 10);
        const DST_MODE_DECREMENT = (1 << 10);
        const DST_MODE_FIXED = (2 << 10);
        const ENABLE = (1<<31);
        const TRIGGER_INTERRUPT = (1<<30);
        const INFINITE_REPEAT = (1<<29);
        const DESTINATION_RELOAD = (1<<12);
        const SOURCE_RELOAD = (1<<15);
        const START_TIMER0 = (0 <<24);
        const START_TIMER1 = (1 <<24);
        const START_TIMER2 = (2 <<24);
        const START_TIMER3 = (3 <<24);
        const START_CARTRIDGE = (4 <<24);
        const START_V_BLANK = (6<<24);
        const START_ARM7_WIFI = (7<<24);
        const START_ARM7_SDMMC = (8<<24);
        const START_ARM7_DSI_WIFI = (9<<24);
        const START_ARM7_WRITE_AES = (10<<24);
        const START_ARM7_READ_AES = (11<<24);
        const START_ARM7_MICROPHONE = (12<<24);
        const START_IMMEDIATE = (16<<24);
        const START_ARM9_H_BLANK = (7<<24);
        const START_ARM9_DISPLAYSYNC = (8<<24);
        const START_ARM9_WORKRAM = (9<<24);
        const START_ARM9_GEOMETRY_ENGINE = (10<<24);
        const START_ARM9_CAMERA = (11<<24);

        const BLOCK_SIZE_1 = (0<<16);
        const BLOCK_SIZE_2 = (1<<16);
        const BLOCK_SIZE_4 = (2<<16);
        const BLOCK_SIZE_8 = (3<<16);
        const BLOCK_SIZE_16 = (4<<16);
        const BLOCK_SIZE_32 = (5<<16);
        const BLOCK_SIZE_64 = (6<<16);
        const BLOCK_SIZE_128 = (7<<16);
        const BLOCK_SIZE_256 = (8<<16);
        const BLOCK_SIZE_512 = (9<<16);
        const BLOCK_SIZE_1024 = (10<<16);
        const BLOCK_SIZE_2048 = (11<<16);
        const BLOCK_SIZE_4096 = (12<<16);
        const BLOCK_SIZE_8192 = (13<<16);
        const BLOCK_SIZE_16384 = (14<<16);
        const BLOCK_SIZE_32768 = (15<<16);
    }

    #[derive(Debug, Clone, Copy)]
    pub struct GlobalControl: u32 {
        const ROUND_ROBIN = (1<<31);
    }
}
impl NDMA {
    pub fn await_channel(&self, channel: usize) {
        while self.channels[channel]
            .control
            .read()
            .contains(Control::ENABLE)
        {}
    }
    pub fn set_fixed_arbitration(&self) {
        unsafe {
            self.global_control.write(GlobalControl::empty());
        }
    }
    pub fn set_round_robin_arbitration(&self) {
        unsafe {
            self.global_control.write(GlobalControl::ROUND_ROBIN);
        }
    }
    pub unsafe fn copy_mem_async(&self, channel: usize, src: &[u32], dst: &mut [u32]) {
        let channel = &self.channels[channel];
        let total_word_count = src.len().min(dst.len());

        channel.src.write(src as *const [u32] as *const u32 as u32);
        channel.dst.write(dst as *mut [u32] as *mut u32 as u32);
        channel.word_count.write(total_word_count as u32);
        channel.block_size.write(total_word_count as u32 >> 2);
        channel.timing.write(0);
        channel.control.write(
            Control::DST_MODE_INCREMENT
                | Control::SRC_MODE_INCREMENT
                | Control::START_IMMEDIATE
                | Control::BLOCK_SIZE_1
                | Control::ENABLE,
        );
    }
    pub fn copy_mem(&self, channel: usize, src: &[u32], dst: &mut [u32]) {
        unsafe { self.copy_mem_async(channel, src, dst) };
        self.await_channel(channel);
    }
    pub unsafe fn set_raw_dma(
        &self,
        channel: usize,
        settings: ChannelConfig,
        source: *const (),
        dest: *mut (),
    ) {
        let ChannelConfig {
            word_count: wc,
            block_size: bs,
            timing: t,
            fill_mode: f,
            control: c,
        } = settings;
        let NDMAChannel {
            src,
            dst,
            word_count,
            block_size,
            timing,
            fill_mode,
            control,
        } = &self.channels[channel];
        unsafe {
            src.write(source as u32);
            dst.write(dest as u32);
            word_count.write(wc);
            block_size.write(bs);
            timing.write(t);
            fill_mode.write(f);
            control.write(c);
        }
    }
}

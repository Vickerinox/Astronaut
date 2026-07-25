// SPDX-FileCopyrightText: 2026 Viktor Karlsson <viktor@koda.re>
// SPDX-License-Identifier: MIT

use crate::MemoryWrapper;
use volatile_register::*;

#[cfg(any(feature = "arm7i", feature = "arm9i"))]
pub const NDMA_HARDWARE: MemoryWrapper<NDMA> = MemoryWrapper(0x4004100 as *mut NDMA);

#[repr(C)]
pub struct NDMA {
    pub global_control: WO<GlobalControl>,
    pub channels: [NDMAChannel; 4],
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
        self.control.write(NDMAControl::empty());
    }
}
#[repr(C)]
pub struct NDMAChannel {
    pub src: WO<u32>,
    pub dst: WO<u32>,
    pub word_count: WO<u32>,
    pub block_size: WO<u32>,
    pub timing: WO<u32>,
    pub fill_mode: WO<u32>,
    pub control: RW<NDMAControl>,
}
#[repr(C)]
pub struct ChannelConfig {
    pub word_count: u32,
    pub block_size: u32,
    pub timing: u32,
    pub fill_mode: u32,
    pub control: NDMAControl,
}

#[repr(u32)]
pub enum NDMAStartMode {
    Timer0 = (0 << 24),
    Timer1 = (1 << 24),
    Timer2 = (2 << 24),
    Timer3 = (3 << 24),
    Cartridge = (4 << 24),
    VBlank = (6 << 24),

    #[cfg(all(feature = "arm7i", not(feature = "arm9i")))]
    Arm7WiFi = (7 << 24),

    #[cfg(all(feature = "arm7i", not(feature = "arm9i")))]
    Arm7SDMMC = (8 << 24),

    #[cfg(all(feature = "arm7i", not(feature = "arm9i")))]
    Arm7DsiWiFi = (9 << 24),

    #[cfg(all(feature = "arm7i", not(feature = "arm9i")))]
    Arm7WriteAES = (10 << 24),

    #[cfg(all(feature = "arm7i", not(feature = "arm9i")))]
    Arm7ReadAES = (11 << 24),

    #[cfg(all(feature = "arm7i", not(feature = "arm9i")))]
    Arm7Microphone = (12 << 24),

    Immediate = (16 << 24),

    #[cfg(all(feature = "arm9i", not(feature = "arm7i")))]
    Arm9HBlank = (7 << 24),

    #[cfg(all(feature = "arm9i", not(feature = "arm7i")))]
    Arm9Displaysync = (8 << 24),

    #[cfg(all(feature = "arm9i", not(feature = "arm7i")))]
    Arm9Workram = (9 << 24),

    #[cfg(all(feature = "arm9i", not(feature = "arm7i")))]
    Arm9GeometryEngine = (10 << 24),

    #[cfg(all(feature = "arm9i", not(feature = "arm7i")))]
    Arm9Camera = (11 << 24),
}
#[repr(u32)]
pub enum SourceMode {
    Increment = (0 << 13),
    Decrement = (1 << 13),
    Fixed = (2 << 13),
    Fill = (3 << 13),
}

#[repr(u32)]
pub enum DestinationMode {
    Increment = (0 << 10),
    Decrement = (1 << 10),
    Fixed = (2 << 10),
}

#[repr(u32)]
pub enum BlockSize {
    Size1 = (0 << 16),
    Size2 = (1 << 16),
    Size4 = (2 << 16),
    Size8 = (3 << 16),
    Size16 = (4 << 16),
    Size32 = (5 << 16),
    Size64 = (6 << 16),
    Size128 = (7 << 16),
    Size256 = (8 << 16),
    Size512 = (9 << 16),
    Size1KiB = (10 << 16),
    Size2KiB = (11 << 16),
    Size4KiB = (12 << 16),
    Size8KiB = (13 << 16),
    Size16KiB = (14 << 16),
    Size32KiB = (15 << 16),
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct NDMAControl: u32 {
        const ENABLE = (1<<31);
        const TRIGGER_INTERRUPT = (1<<30);
        const INFINITE_REPEAT = (1<<29);
        const DESTINATION_RELOAD = (1<<12);
        const SOURCE_RELOAD = (1<<15);
    }

    #[derive(Debug, Clone, Copy)]
    pub struct GlobalControl: u32 {
        const ROUND_ROBIN = (1<<31);
    }
}
impl NDMAControl {
    const SRC_MODE_MASK: u32 = (3 << 13);
    const DST_MODE_MASK: u32 = (3 << 10);
    const BLOCK_SIZE_MASK: u32 = (0xF << 16);
    const START_MASK: u32 = (0x1F << 24);

    pub const fn with_start_mode(mut self, start_mode: NDMAStartMode) -> Self {
        Self::from_bits_retain((self.bits() & !Self::START_MASK) | start_mode as u32)
    }

    pub const fn with_src_mode(mut self, src_mode: SourceMode) -> Self {
        Self::from_bits_retain((self.bits() & !Self::SRC_MODE_MASK) | src_mode as u32)
    }

    pub const fn with_dst_mode(mut self, dst_mode: DestinationMode) -> Self {
        Self::from_bits_retain((self.bits() & !Self::DST_MODE_MASK) | dst_mode as u32)
    }

    pub const fn with_block_size(mut self, block_size: BlockSize) -> Self {
        Self::from_bits_retain((self.bits() & !Self::BLOCK_SIZE_MASK) | block_size as u32)
    }
}
impl NDMA {
    pub fn await_channel(&self, channel: usize) {
        while self.channels[channel]
            .control
            .read()
            .contains(NDMAControl::ENABLE)
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
            NDMAControl::ENABLE
                .with_start_mode(NDMAStartMode::Immediate)
                .with_block_size(BlockSize::Size1)
                .with_dst_mode(DestinationMode::Increment)
                .with_src_mode(SourceMode::Increment),
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

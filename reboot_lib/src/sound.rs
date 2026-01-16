use crate::{
    spi::{
        touchscreen::{cdc_write_reg, CdcReg, CntReg},
        write_powerman, Control, SPI_HARDWARE,
    },
    MemoryWrapper,
};
use bitflags::bitflags;
use volatile_register::*;
pub const SOUND_HARDWARE: MemoryWrapper<NTRSoundRegisters> =
    MemoryWrapper(0x4000400 as *mut NTRSoundRegisters);

#[repr(C)]
pub struct NTRSoundRegisters {
    pub channels: [SoundChannel; 16],
    pub master_control: RW<u32>,
    pub bias: RW<u32>,
    pub capture_0: RW<u8>,
    pub capture_1: RW<u8>,
    _0x10a: [u8; 6],
    pub capture_0_destination: RW<u32>,
    pub capture_0_len: RW<u32>,
    pub capture_1_destination: RW<u32>,
    pub capture_1_len: RW<u32>,
    _0x120: [u32; 0x38],
    _0x200: [u8; 0x4000],
    pub dsi_mic_control: RW<u16>,
    _0x4202: u16,
    pub dsi_mic_data: RW<u32>,
    _0x4204: [u32; 0x3E],
    pub dsi_sound_control: RW<u16>,
}
pub struct TWLSoundRegisters {
    pub dsi_mic_control: RW<u16>,
    _0x2: u16,
    pub dsi_mic_data: RW<u32>,
    _0x4: [u8; 0xF8],
    pub dsi_sound_control: RW<u16>,
}
bitflags::bitflags! {
    pub struct TWLSoundCnt: u16 {
        const SOUND_RATIO_0 = 0;
        const SOUND_RATIO_1 = 1;
        const SOUND_RATIO_2 = 2;
        const SOUND_RATIO_3 = 3;
        const SOUND_RATIO_4 = 4;
        const SOUND_RATIO_5 = 5;
        const SOUND_RATIO_6 = 6;
        const SOUND_RATIO_7 = 7;
        const SOUND_RATIO_8 = 8;

        const HIGH_SOUND_FREQ = (1<<13);
        const MUTE = (1<<14);
        const ENABLE = (1<<15);
    }
}
impl NTRSoundRegisters {
    pub fn init(&self) {
        unsafe {
            self.master_control.write((1 << 15) | 0x7f);
            self.capture_0.write(0);
            self.capture_1.write(0);
            self.bias.write(0x200);
            self.dsi_sound_control.write(4 | (1 << 13));
            cdc_write_reg(CdcReg::Control(CntReg::PllJ), 15);
            cdc_write_reg(CdcReg::Control(CntReg::DacNdac), 0x85);
            cdc_write_reg(CdcReg::Control(CntReg::AdcNadc), 0x85);

            self.dsi_sound_control.modify(|i| i | 0x8000);
            //self.master_control.write((1<<15));
        }
        self.clear_channels();
    }
    pub fn clear_channels(&self) {
        for channel in &self.channels {
            unsafe {
                channel.control.write(SoundControl::empty());
                channel.source.write(0);
                channel.timer.write(0);
                channel.loop_start.write(0);
                channel.length.write(0);
            }
        }
    }
}
#[repr(C)]
pub struct SoundChannel {
    pub control: RW<SoundControl>,
    pub source: WO<u32>,
    pub timer: WO<u16>,
    pub loop_start: WO<u16>,
    pub length: WO<u32>,
}
impl SoundChannel {
    pub unsafe fn start_test_beep(&self) {
        self.timer.write(timer_from_freq(440));
        self.control
            .write(SoundControl::new().with_repeat_mode(RepeatMode::Infinite));
    }
}
pub const fn timer_from_freq(freq: u32) -> u16 {
    0xFFFF - ((33513982 / 2) / freq) as u16
}
bitflags! {
    #[derive(Clone, Copy, Default)]
    pub struct SoundControl: u32 {
        const HOLD = (1<<15);
        const FORMAT_PCM8 = (0<<29);
        const FORMAT_PCM16 = (1<<29);
        const FORMAT_ADPCM = (2<<29);
        const FORMAT_PSG = (3<<29);
        const REPEAT_MANUAL = (0<<27);
        const REPEAT_INFINITE = (1<<27);
        const REPEAT_ONESHOT = (2<<27);
        const START = (1<<31);
    }
}
#[repr(u8)]
pub enum SoundFormat {
    PCM8 = 0,
    PCM16 = 1,
    ADPCM = 2,
    PSG = 3,
}
#[repr(u8)]
pub enum RepeatMode {
    Manual = 0,
    Infinite = 1,
    Oneshot = 2,
}

impl SoundControl {
    pub const fn new() -> Self {
        Self::START
            .with_repeat_mode(RepeatMode::Oneshot)
            .with_sound_format(SoundFormat::PSG)
            .with_panning(64)
            .with_volume(127)
    }
    pub const fn with_repeat_mode(self, repeat_mode: RepeatMode) -> Self {
        Self::from_bits_retain(((repeat_mode as u8 as u32) << 27) | (self.bits() & !(3 << 27)))
    }
    pub const fn with_sound_format(self, format: SoundFormat) -> Self {
        Self::from_bits_retain(((format as u8 as u32) << 29) | (self.bits() & !(3 << 29)))
    }
    pub const fn with_volume(self, volume: u8) -> Self {
        Self::from_bits_retain((volume as u32 & 0x7F) | (self.bits() & !(0x7f)))
    }
    pub const fn with_panning(self, panning: u8) -> Self {
        Self::from_bits_retain(((panning as u32 & 0x7F) << 16) | (self.bits() & !(0x7f0000)))
    }
    pub const fn volume(&self) -> u8 {
        (self.bits() & 0x7f) as u8
    }
}

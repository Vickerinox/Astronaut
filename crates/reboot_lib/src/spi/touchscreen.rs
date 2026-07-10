use crate::spi::{write_powerman, Control, SPI_HARDWARE};

use super::SPIControl;
struct RawTouchData {}

/*
pub struct TouchReadError;
unsafe fn touch_read_data() -> Result<RawTouchData, TouchReadError> {
    crate::critical_function(||{

    });
}
*/

#[repr(u8)]
#[derive(Clone, Copy)]
pub enum CdcReg {
    Control(CntReg),       //= 0x00, //< Chip control
    Sound(SndReg),         //= 0x01, //< ADC/DAC control
    TouchCnt(TouchCntReg), //    = 0x03, //< TSC control
    AdcCoefficients(u8),
    BufferModeData(u8),

    UndocumentedReset,
    TSCNDSMode,
}
impl CdcReg {
    pub const fn as_bank_and_reg(self) -> (u8, u8) {
        match self {
            Self::Control(reg) => (0, reg as u8),
            Self::Sound(reg) => (1, reg as u8),
            Self::TouchCnt(reg) => (3, reg as u8),
            Self::AdcCoefficients(reg) => (4, reg),
            Self::BufferModeData(reg) => (0xFC, reg),
            CdcReg::UndocumentedReset => (0x63, 0),
            CdcReg::TSCNDSMode => (0xFF, 5),
        }
    }
}

impl Into<CdcReg> for CntReg {
    fn into(self) -> CdcReg {
        CdcReg::Control(self)
    }
}
impl Into<CdcReg> for SndReg {
    fn into(self) -> CdcReg {
        CdcReg::Sound(self)
    }
}
impl Into<CdcReg> for TouchCntReg {
    fn into(self) -> CdcReg {
        CdcReg::TouchCnt(self)
    }
}

bitflags::bitflags! {
    pub struct SARADCCnt1: u8 {
        const POWER_DOWN = 0x80;

        const RESOLUTION_8BIT = (1<<5);
        const RESOLUTION_10BIT = (2<<5);
        const RESOLUTION_12BIT = (3<<5);

        const CLOCK_DIV1 = (0<<3); //only for use with 8-bit mode
        const CLOCK_DIV2 = (1<<3); //only for use with 8/10-bit mode
        const CLOCK_DIV4 = (2<<3); //recommended for 8/10-bit mode
        const CLOCK_DIV8 = (3<<3); //recommended for 12-bit mode (i.e standard)

        const AVERAGE_USE_MEAN = (0<<2);
        const AVERAGE_USE_MEDIAN = (1<<2);

        const AVERAGING_NON = 0;
        const AVERAGING_LOW = 1;
        const AVERAGING_MED = 2;
        const AVERAGING_HIG = 3;
    }
    pub struct SARADCCnt2: u8 {
        const HOST_CONTROLLED_CONVERSION = 0;
        const SELF_CONTROLLED_CONVERSION = 0x80;

        const MODE_NONE = (0<<2);
        const MODE_XY = (1<<2);
        const MODE_XYZ = (2<<2);
        const MODE_X = (3<<2);
        const MODE_Y = (4<<2);
        const MODE_Z = (5<<2);
        const MODE_VBAT = (6<<2);
        const MODE_AUX2 = (7<<2);
        const MODE_AUX1 = (8<<2);
        const MODE_AUTO = (9<<2);
        const MODE_TEMP1 = (10<<2);
        const MODE_PORTSCAN = (11<<2);
        const MODE_TEMP2 = (12<<2);
    }

    pub struct PrechargeSense: u8 {
        const DISABLE_PEN_SENSE = 0x80;
        const PRECHARGE_QUARTER_US = (0<<4);
        const PRECHARGE_1_US = (1<<4);
        const PRECHARGE_3_US = (2<<4);
        const PRECHARGE_10_US = (3<<4);
        const PRECHARGE_30_US = (4<<4);
        const PRECHARGE_100_US = (5<<4);
        const PRECHARGE_300_US = (6<<4);
        const PRECHARGE_1000_US = (7<<4);
        const SENSE_TIME_1_US = 0;
        const SENSE_TIME_2_US = 1;
        const SENSE_TIME_3_US = 2;
        const SENSE_TIME_10_US = 3;
        const SENSE_TIME_30_US = 4;
        const SENSE_TIME_100_US = 5;
        const SENSE_TIME_300_US = 6;
        const SENSE_TIME_1000_US = 7;
    }
}
#[repr(u8)]
#[derive(Clone, Copy)]
pub enum TouchCntReg {
    SarAdcCnt1 = 0x02,
    SarAdcCnt2 = 0x03,
    PrechargeSense = 0x04,
    PanelVoltageStabilization = 0x05,
    Status = 0x09,
    BufferMode = 0x0D,
    TwlBufferMode = 0x0E,
    ScanModeTimer = 0x0F,
    ScanModeTimerClock = 0x10,
    SarAdcClock = 0x11,
    DebouncePenup = 0x12,

    DebouncePendown = 0x14,
}
#[repr(u8)]
#[derive(Clone, Copy)]
pub enum CntReg {
    //TSC timing and PLL controls
    Reset = 0x01,
    OverTemp = 0x03,
    ClockMux = 0x04,
    PllPr = 0x05,
    PllJ = 0x06,
    PllD16 = 0x07,
    DacNdac = 0x0B,
    DacMdac = 0x0C,
    DOSRMSB = 0x0D,
    DOSRLSB = 0x0E,
    IDAC = 0x0F,
    Interpolation = 0x10,

    AdcNadc = 0x12,
    AdcMadc = 0x13,
    AOSR = 0x14,
    IADC = 0x15,
    Decimation = 0x16,

    ClkoutMux = 0x19,
    ClkDivM = 0x1A,

    //TSC CODEC control
    ClkDivN = 0x1E,

    //TSC status and interrupt flags
    AdcFlags = 0x24,

    //TSC pin control
    GPIO1Control = 0x33,
    GPIO2Control = 0x34,
    SdOut = 0x35,
    SdIn = 0x36,
    MISO = 0x37,
    SCLK = 0x38,

    GPIO12 = 0x39,
    GPIO3Pin = 0x3A,

    //TSC DAC/ADC and beep
    DacInstructionSet = 0x3C,
    AdcInstructionSet = 0x3D,

    DacCtrl = 0x3F,
    DacVolume = 0x40,
    DacVolumeLeft = 0x41,
    DacVolumeRight = 0x42,

    DacControl1 = 0x44,
    DacControl2 = 0x45,

    DacBeep1 = 0x47,
    DacBeep2 = 0x48,
    DacBeepLen1 = 0x49,
    DacBeepLen2 = 0x4A,
    DacBeepLen3 = 0x4B,
    DacBeepSinMSB = 0x4C,
    DacBeepSinLSB = 0x4D,
    DacBeepCosMSB = 0x4E,
    DacBeepCosLSB = 0x4F,
    AdcMic = 0x51,
    AdcVolFine = 0x52,
    AdcVolCoarse = 0x53,

    // TSC AGC and ADC
    AGCMaxGain = 0x58,
    VolSarAdc = 0x74,
    VolGain = 0x75,
}
#[repr(u8)]
#[derive(Clone, Copy)]
pub enum SndReg {
    AmplifierError = 0x1E,
    HeadphoneDriver = 0x1F,
    ClassDSpeakerAmp = 0x20,
    PopRemovalSetting = 0x21,
    RampDownPeriod = 0x22,
    DacMixerRouting = 0x23,
    VolumeHPL = 0x24,
    VolumeHPR = 0x25,
    VolumeSPL = 0x26,
    VolumeSPR = 0x27,

    DriverHPL = 0x28,
    DriverHPR = 0x29,
    DriverSPL = 0x2A,
    DriverSPR = 0x2B,

    DriverCnt = 0x2C,

    MicBias = 0x2E,
    MicGain = 0x2F,

    FineGain = 0x30,
    InputSelection = 0x31,
    CmSetting = 0x32,
}
pub unsafe fn read_tsc_var(command: u8, data: &mut [u16]) {
    let mut iter = data.iter_mut();
    let Some(mut current) = iter.next() else {
        return;
    };

    SPI_HARDWARE
        .set_control(SPIControl::ENABLE | SPIControl::DEVICE_TOUCHSCR | SPIControl::SELECT_HOLD);
    SPI_HARDWARE.write_value(command);
    SPI_HARDWARE.read_value();
    SPI_HARDWARE.write_value(command);

    while let Some(next) = iter.next() {
        let msb = super::SPI_HARDWARE.read_value();
        let lsb = super::SPI_HARDWARE.exchange_raw_value(command);

        *current = (((msb as u16) << 5) | ((lsb as u16) >> 5)) & 0xFFF;
        current = next;
    }
    let msb = super::SPI_HARDWARE.read_value();
    super::SPI_HARDWARE.set_control(SPIControl::ENABLE | SPIControl::DEVICE_TOUCHSCR);
    let lsb = super::SPI_HARDWARE.read_value();
    *current = (((msb as u16) << 5) | ((lsb as u16) >> 5)) & 0xFFF;
}
fn i_sort(vals: &mut [u16; 5]) {
    //sort our samples using insertion sort
    for i in 1.. {
        let Some(x) = vals.get(i).copied() else { break };
        let mut j = i;

        while let Some(val) = vals.get(j - 1).copied() {
            if val > x {
                vals[j] = val;
                j -= 1
            } else {
                break;
            }
        }
        vals[j] = x;
    }
}
fn find_best_read(mut vals: [u16; 5]) -> u16 {
    i_sort(&mut vals);
    //find a most likely good read by trying to find 3 matching values

    //check middle 3 values for equal match
    let mut slic = &vals[1..4];
    let mut dif = vals[3] - vals[1];
    if dif == 0 {
        return vals[2];
    }

    //check lower 3 for equal match
    let tmp = vals[2] - vals[0];
    if tmp == 0 {
        return vals[1];
    } else if tmp < dif {
        slic = &vals[0..3];
        dif = tmp;
    }

    //check upper 3 for equal match
    let tmp = vals[4] - vals[2];
    if tmp == 0 {
        return vals[3];
    } else if tmp < dif {
        slic = &vals[2..5];
    }

    //if none of the ranges had matching values, create a weighted average instead
    let sum = (slic[0] * 5) + (slic[1] * 6) + (slic[2] * 5);
    sum / 16
}

pub unsafe fn read_tsc_pos_cdc() -> Option<(u16, u16)> {
    let mut raw_data = [0u8; 40];
    crate::critical_function(|| {
        cdc_read_array(CdcReg::BufferModeData(1), &mut raw_data);
    });

    let mut x_values = [0u16; 5];
    let mut y_values = [0u16; 5];

    for i in 0..5 {
        let x = u16::from_le_bytes([raw_data[(i * 2) + 1], raw_data[i * 2]]);
        let y = u16::from_le_bytes([raw_data[(i * 2) + 11], raw_data[(i * 2) + 10]]);
        if (x | y) & 0xF000 > 0 {
            return None;
        }
        x_values[i] = x;
        y_values[i] = y;
    }

    Some((find_best_read(x_values), find_best_read(y_values)))
}
pub unsafe fn read_tsc_pos_tsc() -> Option<(u16, u16)> {
    //let mut raw_data = [0u8; 20];
    let mut raw_data = [0u16; 20];
    crate::critical_function(|| {
        read_tsc_var(0x85 | 0x30, &mut raw_data[0..5]);
        read_tsc_var(0x85 | 0x40, &mut raw_data[5..10]);
        read_tsc_var(0x85 | 0x50, &mut raw_data[10..15]);
        read_tsc_var(0x85 | 0x10, &mut raw_data[15..20]);
        //cdc_read_array(CdcReg::BufferModeData(1), &mut raw_data);
    });
    let mut rawx = 0;
    let mut rawy = 0;

    for i in 0..5 {
        rawx += raw_data[10 + i];
        rawy += raw_data[15 + i];
    }
    Some((rawx / 5, rawy / 5))
}
pub unsafe fn init_tsc_dsi() {
    (0x04004000 as *mut u16).write_volatile(0x101);
    (0x04004004 as *mut u16)
        .write_volatile((1 << 0) | (1 << 1) | (1 << 2) | (1 << 7) | (1 << 8) | (1 << 0));
    (0x04004008 as *mut u32).write_volatile(0x93FFFB06);

    (0x04004012 as *mut u16).write_volatile(0x1988);
    (0x04004014 as *mut u16).write_volatile(0x264C);
    //(0x04004C02 as *mut u16).write_volatile(0x4000);

    core::ptr::write_volatile(0x4004C00 as *mut u16, 0x8080);

    cdc_write_reg(CdcReg::Control(CntReg::Reset), 1);
    crate::swi_delay(0x20BA); //Wait 1ms for reset (recommended by TSC2117 technical sheet, but never done by anyone else?)

    const INIT_LIST: &[(CdcReg, u8)] = &[
        (CdcReg::TouchCnt(TouchCntReg::SarAdcCnt1), 0x80),
        (CdcReg::Control(CntReg::GPIO12), 0x66), //GPI2 -> enable & HP_SP + GPI1 -> enable & reserved?
        (CdcReg::Sound(SndReg::ClassDSpeakerAmp), 0x16),
        (CdcReg::Control(CntReg::ClockMux), 0),
        (CdcReg::Control(CntReg::AdcNadc), 0x81),
        (CdcReg::Control(CntReg::AdcMadc), 0x82),
        (CdcReg::Control(CntReg::AdcMic), 0x82),
        (CdcReg::Control(CntReg::AdcMic), 0),
        (CdcReg::Control(CntReg::ClockMux), 3),
        (CdcReg::Control(CntReg::PllPr), 0xA1),
        (CdcReg::Control(CntReg::PllJ), 0x15),
        (CdcReg::Control(CntReg::DacNdac), 0x87),
        (CdcReg::Control(CntReg::DacMdac), 0x83),
        (CdcReg::Control(CntReg::AdcNadc), 0x87),
        (CdcReg::Control(CntReg::AdcMadc), 0x83),
        (CdcReg::TouchCnt(TouchCntReg::ScanModeTimerClock), 0x88), // Use external clock + divide by 8
        (CdcReg::Control(CntReg::DacVolumeLeft), 8),
        (CdcReg::Control(CntReg::DacVolumeRight), 8),
        (CdcReg::Control(CntReg::GPIO3Pin), 0),
        (CdcReg::AdcCoefficients(0x8), 0x7F),
        (CdcReg::AdcCoefficients(0x9), 0xE1),
        (CdcReg::AdcCoefficients(0xA), 0x80),
        (CdcReg::AdcCoefficients(0xB), 0x1F),
        (CdcReg::AdcCoefficients(0xC), 0x7F),
        (CdcReg::AdcCoefficients(0xD), 0xC1),
        (CdcReg::Sound(SndReg::MicGain), 0x2B),
        (CdcReg::Sound(SndReg::FineGain), 0x40),
        (CdcReg::Sound(SndReg::InputSelection), 0x40),
        (CdcReg::Sound(SndReg::CmSetting), 0x60),
        (CdcReg::Control(CntReg::VolSarAdc), 0xD2), // 62HZ throughput + DAC ON + 1bit hysterisis
        (CdcReg::Sound(SndReg::PopRemovalSetting), 0x20), //15MS pop removal (awesome)
        (CdcReg::Sound(SndReg::RampDownPeriod), 0xF0),
        (CdcReg::Control(CntReg::DacCtrl), 0xD4), // Power up DACs + route R->R + route L->L
        (CdcReg::Sound(SndReg::DacMixerRouting), 0x44), // Route DAC to matching AMPS on both L/R channels
        (CdcReg::Sound(SndReg::HeadphoneDriver), 0xD4), // output common-mode voltage 1.65V + turn on HPL/HPR
        (CdcReg::Sound(SndReg::DriverHPL), 0x4E), //set HPL to 9dB PGA + unmute + use high impedance powerdown
        (CdcReg::Sound(SndReg::DriverHPR), 0x4E), //set HPR to 9dB PGA + unmute + use high impedance powerdown
        (CdcReg::Sound(SndReg::VolumeHPL), 0x9E), //Route L volume to HPR + incorrect reset value?
        (CdcReg::Sound(SndReg::VolumeHPR), 0x9E), //Route R volume to HPR + incorrect reset value?
        (CdcReg::Sound(SndReg::ClassDSpeakerAmp), 0xD4), //Enable amp + set incorrect reset?
        (CdcReg::Sound(SndReg::DriverSPL), 0x14), //enable L channel + 18dB stage gain
        (CdcReg::Sound(SndReg::DriverSPR), 0x14), //enable R channel + 18dB stage gain
        (CdcReg::Sound(SndReg::VolumeSPL), 0xA7), //Route channel L to amp and set volume gain
        (CdcReg::Sound(SndReg::VolumeSPR), 0xA7), //Route channel R to amp and set volume gain
        (CdcReg::Control(CntReg::DacVolume), 0), //unmute L/R DACs and let them have independent control
        (CdcReg::Control(CntReg::GPIO3Pin), 0x60), //enable + reserved bit?
        (CdcReg::TouchCnt(TouchCntReg::SarAdcCnt1), 0x0),
        (CdcReg::Control(CntReg::PllJ), 0x0),
        (CdcReg::TouchCnt(TouchCntReg::TwlBufferMode), 0),
        (CdcReg::TouchCnt(TouchCntReg::SarAdcCnt1), 0x18),
        (CdcReg::TouchCnt(TouchCntReg::SarAdcCnt2), 0x8B),
        (CdcReg::TouchCnt(TouchCntReg::ScanModeTimer), 0xA0),
        (CdcReg::TouchCnt(TouchCntReg::PanelVoltageStabilization), 4),
        (CdcReg::TouchCnt(TouchCntReg::PrechargeSense), 0x22),
        (CdcReg::TouchCnt(TouchCntReg::DebouncePenup), 0),
        (
            CdcReg::TouchCnt(TouchCntReg::TwlBufferMode),
            0x80 | (5 << 3),
        ),
    ];
    for (reg, value) in INIT_LIST {
        cdc_write_reg(reg.clone(), *value);
    }
}

pub unsafe fn cdc_write_mask(reg: impl Into<CdcReg>, mask: u8, value: u8) {
    let (bank, reg) = reg.into().as_bank_and_reg();
    bank_switch_tsc(bank);
    let original = read_tsc(reg);
    write_tsc(reg, (original & !mask) | (value & mask));
    super::SPI_HARDWARE.wait_busy();
}

pub unsafe fn is_pen_down() -> bool {
    (cdc_read_reg(TouchCntReg::Status) & 0x80 > 0) //&&(cdc_read_reg(TouchCntReg::TwlBufferMode) & 2 == 0)
}

unsafe fn bank_switch_tsc(bank: u8) {
    //let write = if CUR_BANK == 0xff { 0x7F } else { 0 };
    write_tsc(0, bank);
}
unsafe fn cdc_read_reg(reg: impl Into<CdcReg>) -> u8 {
    let (bank, reg) = reg.into().as_bank_and_reg();
    super::SPI_HARDWARE.wait_busy();
    bank_switch_tsc(bank);
    read_tsc(reg)
}
pub unsafe fn cdc_write_reg(reg: CdcReg, value: u8) {
    let (bank, reg) = reg.as_bank_and_reg();
    super::SPI_HARDWARE.wait_busy();
    bank_switch_tsc(bank);
    write_tsc(reg, value);
    super::SPI_HARDWARE.wait_busy();
}

/*
pub unsafe fn cdc_write_array(start_reg: impl Into<CdcRegister>, data: &[u8]) {
    let (bank, reg) = start_reg.into().as_bank_and_reg();
    bank_switch_tsc(bank);
    super::SPI_HARDWARE
        .set_control(SPIControl::ENABLE | SPIControl::DEVICE_CODEC | SPIControl::SELECT_HOLD);
    super::SPI_HARDWARE.write_value(reg << 1);
    for byte in data {
        super::SPI_HARDWARE.write_value(*byte);
    }
    super::SPI_HARDWARE.set_control(SPIControl::DISABLE);
}
    */
pub unsafe fn cdc_read_array(start_reg: CdcReg, data: &mut [u8]) {
    let mut iter = data.iter_mut();
    let Some(mut current) = iter.next() else {
        return;
    };

    let (bank, reg) = start_reg.as_bank_and_reg();
    bank_switch_tsc(bank);
    super::SPI_HARDWARE.wait_busy();
    super::SPI_HARDWARE
        .set_control(SPIControl::ENABLE | SPIControl::DEVICE_CODEC | SPIControl::SELECT_HOLD);
    super::SPI_HARDWARE.write_value((reg << 1) | 1);
    while let Some(next) = iter.next() {
        *current = super::SPI_HARDWARE.read_value();
        current = next;
    }
    super::SPI_HARDWARE.set_control(SPIControl::ENABLE | SPIControl::DEVICE_CODEC);
    *current = super::SPI_HARDWARE.read_value();
}
pub unsafe fn write_tsc(reg: u8, value: u8) {
    super::SPI_HARDWARE.wait_busy();
    super::SPI_HARDWARE
        .set_control(SPIControl::ENABLE | SPIControl::DEVICE_CODEC | SPIControl::SELECT_HOLD);
    super::SPI_HARDWARE.write_value(reg << 1);
    super::SPI_HARDWARE.set_control(SPIControl::ENABLE | SPIControl::DEVICE_CODEC);
    super::SPI_HARDWARE.write_value(value);
}
pub unsafe fn read_tsc(reg: u8) -> u8 {
    super::SPI_HARDWARE.wait_busy();
    super::SPI_HARDWARE
        .set_control(SPIControl::ENABLE | SPIControl::DEVICE_CODEC | SPIControl::SELECT_HOLD);
    super::SPI_HARDWARE.write_value(reg << 1 | 1);
    super::SPI_HARDWARE.set_control(SPIControl::ENABLE | SPIControl::DEVICE_CODEC);
    super::SPI_HARDWARE.read_value()
}

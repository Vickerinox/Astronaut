use crate::spi::SPI_HARDWARE;

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
}
impl CdcReg {
    pub const fn as_bank_and_reg(self) -> (u8, u8) {
        match self {
            Self::Control(reg) => (0, reg as u8),
            Self::Sound(reg) => (1, reg as u8),
            Self::TouchCnt(reg) => (3, reg as u8),
            Self::AdcCoefficients(reg) => (4, reg),
            Self::BufferModeData(reg) => (0xFC, reg),
            //CdcRegister::TOUCHDATA => todo!(),
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
#[repr(u8)]
#[derive(Clone, Copy)]
pub enum TouchCntReg {
    SarAdcCnt1 = 0x02,
    SarAdcCnt2 = 0x03,
    PrechargeSense = 0x04,
    PanelVoltageStabilization = 0x05,
    Status = 0x09,
    TwlPenDown = 0x0E,
    ScanModeTimer = 0x0F,
    ScanModeTimerClock = 0x10,
    SarAdcClock = 0x11,
    DebouncePenup = 0x12,

    DebouncePendown = 0x14,
}
#[repr(u8)]
#[derive(Clone, Copy)]
pub enum CntReg {
    Reset = 0x01,
    ClockMux = 0x04,
    PllPr = 0x05,
    PllJ = 0x06,
    PllD16 = 0x07,
    DacNdac = 0x0B,
    DacMdac = 0x0C,
    AdcNadc = 0x12,
    AdcMadc = 0x13,
    ClkoutMux = 0x19,

    GPIO1Control = 0x33,
    GPIO2Control = 0x34,

    GPIO3Pin = 0x3A,

    NocashAdcDcMeasurement1 = 0x39,
    DacCtrl = 0x3F,
    DacVolume = 0x40,
    DacVolumeLeft = 0x41,
    DacVolumeRight = 0x42,
    DacBeep1 = 0x47,
    DacBeep2 = 0x48,
    DacBeepLen24 = 0x49,
    DacBeepSin16 = 0x4C,
    DacBeepCos16 = 0x4E,
    AdcMic = 0x51,
    AdcVolFine = 0x52,
    AdcVolCoarse = 0x53,

    SarAdc = 0x74,
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
pub unsafe fn init_tsc() {
    const INIT_LIST: &[(CdcReg, u8)] = &[
        (CdcReg::Control(CntReg::Reset), 1),
        (CdcReg::Control(CntReg::NocashAdcDcMeasurement1), 0x66),
        (CdcReg::Sound(SndReg::ClassDSpeakerAmp), 0x10),
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
        (CdcReg::TouchCnt(TouchCntReg::ScanModeTimerClock), 0x88),
        (CdcReg::AdcCoefficients(0x8), 0x7F),
        (CdcReg::AdcCoefficients(0x9), 0xE1),
        (CdcReg::AdcCoefficients(0xA), 0x80),
        (CdcReg::AdcCoefficients(0xB), 0x1F),
        (CdcReg::AdcCoefficients(0xC), 0x7F),
        (CdcReg::AdcCoefficients(0xD), 0xC1),
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
        (CdcReg::Control(CntReg::SarAdc), 0x82),
        (CdcReg::Control(CntReg::SarAdc), 0x92),
        (CdcReg::Control(CntReg::SarAdc), 0xD2),
        (CdcReg::Sound(SndReg::PopRemovalSetting), 0x20),
        (CdcReg::Sound(SndReg::RampDownPeriod), 0xF0),
        (CdcReg::Control(CntReg::DacCtrl), 0xD4),
        (CdcReg::Sound(SndReg::DacMixerRouting), 0x44),
        (CdcReg::Sound(SndReg::HeadphoneDriver), 0xD4),
        (CdcReg::Sound(SndReg::DriverHPL), 0x4E),
        (CdcReg::Sound(SndReg::DriverHPR), 0x4E),
        (CdcReg::Sound(SndReg::VolumeHPL), 0x9E),
        (CdcReg::Sound(SndReg::VolumeHPR), 0x9E),
        (CdcReg::Sound(SndReg::ClassDSpeakerAmp), 0xD4),
        (CdcReg::Sound(SndReg::DriverSPL), 0x14),
        (CdcReg::Sound(SndReg::DriverSPR), 0x14),
        (CdcReg::Sound(SndReg::VolumeSPL), 0xA7),
        (CdcReg::Sound(SndReg::VolumeSPR), 0xA7),
    ];
    for (reg, value) in INIT_LIST {
        cdc_write_reg(reg.clone(), *value);
    }
    //BANAN
    /*
    cdc_write_reg(CntReg::Reset, 1);
    cdc_write_reg(CntReg::NocashAdcDcMeasurement1, 0x66);
    cdc_write_reg(SndReg::ClassDSpeakerAmp, 0x16);
    cdc_write_reg(CntReg::ClockMux, 0);

    cdc_write_reg(CntReg::AdcNadc   , 0x81);
    cdc_write_reg(CntReg::AdcMadc   , 0x82);
    cdc_write_reg(CntReg::AdcMic    , 0x82);
    cdc_write_reg(CntReg::AdcMic    , 0     );
    cdc_write_reg(CntReg::ClockMux  , 3     );

    cdc_write_reg(CntReg::PllPr     ,  0xA1);
    cdc_write_reg(CntReg::PllJ      ,  0x15);
    cdc_write_reg(CntReg::DacNdac   ,  0x87);
    cdc_write_reg(CntReg::DacMdac   ,  0x83);
    cdc_write_reg(CntReg::AdcNadc   ,  0x87);
    cdc_write_reg(CntReg::AdcMadc   ,  0x83);

    cdc_write_reg(TouchCntReg::ScanModeTimerClock, 0x88);

    //sound init?
    cdc_write_reg(CdcRegister::AdcCoefficients(0x8), 0x7F);
    cdc_write_reg(CdcRegister::AdcCoefficients(0x9), 0xE1);
    cdc_write_reg(CdcRegister::AdcCoefficients(0xA), 0x80);
    cdc_write_reg(CdcRegister::AdcCoefficients(0xB), 0x1F);
    cdc_write_reg(CdcRegister::AdcCoefficients(0xC), 0x7F);
    cdc_write_reg(CdcRegister::AdcCoefficients(0xD), 0xC1);

    cdc_write_reg(CntReg::DacVolumeLeft , 8);
    cdc_write_reg(CntReg::DacVolumeRight, 8);
    cdc_write_reg(CntReg::GPIO3Pin      , 0);

    cdc_write_reg(CdcRegister::AdcCoefficients(0x8), 0x7F);
    cdc_write_reg(CdcRegister::AdcCoefficients(0x9), 0xE1);
    cdc_write_reg(CdcRegister::AdcCoefficients(0xA), 0x80);
    cdc_write_reg(CdcRegister::AdcCoefficients(0xB), 0x1F);
    cdc_write_reg(CdcRegister::AdcCoefficients(0xC), 0x7F);
    cdc_write_reg(CdcRegister::AdcCoefficients(0xD), 0xC1);

    cdc_write_reg(SndReg::MicGain       , 0x2B);
    cdc_write_reg(SndReg::FineGain      , 0x40);
    cdc_write_reg(SndReg::InputSelection, 0x40);
    cdc_write_reg(SndReg::CmSetting     , 0x60);

    cdc_write_reg(CntReg::SarAdc, 0x82);
    cdc_write_reg(CntReg::SarAdc, 0x92);
    cdc_write_reg(CntReg::SarAdc, 0xD2);

    cdc_write_reg(SndReg::PopRemovalSetting , 0x20);
    cdc_write_reg(SndReg::RampDownPeriod    , 0xF0);
    cdc_write_reg(CntReg::DacCtrl           , 0xD4);

    cdc_write_reg(SndReg::DacMixerRouting   , 0x44);
    cdc_write_reg(SndReg::HeadphoneDriver   , 0xD4);
    cdc_write_reg(SndReg::DriverHPL         , 0x4E);
    cdc_write_reg(SndReg::DriverHPR         , 0x4E);
    cdc_write_reg(SndReg::VolumeHPL         , 0x9E);
    cdc_write_reg(SndReg::VolumeHPR         , 0x9E);
    cdc_write_reg(SndReg::ClassDSpeakerAmp  , 0xD4);
    cdc_write_reg(SndReg::DriverSPL         , 0x14);
    cdc_write_reg(SndReg::DriverSPR         , 0x14);
    cdc_write_reg(SndReg::VolumeSPL         , 0xA7);
    cdc_write_reg(SndReg::VolumeSPR         , 0xA7);
    */
    cdc_write_reg(CdcReg::Control(CntReg::DacVolume), 0);
    core::ptr::write_volatile(0x4004C00 as *mut u16, 0x8080);
    cdc_write_reg(CdcReg::Control(CntReg::GPIO3Pin), 0x60);

    cdc_read_reg(CdcReg::TouchCnt(TouchCntReg::SarAdcCnt1));
    cdc_write_reg(CdcReg::TouchCnt(TouchCntReg::SarAdcCnt1), 0);

    
}
pub unsafe fn enable_tsc() {

    //ENABLE?
    cdc_write_mask(CdcReg::TouchCnt(TouchCntReg::TwlPenDown), 0x80, 0);
    cdc_write_mask(CdcReg::TouchCnt(TouchCntReg::SarAdcCnt1), 0x18, 3 << 3);

    cdc_write_reg(CdcReg::TouchCnt(TouchCntReg::ScanModeTimer), 0xA0);
    cdc_write_mask(CdcReg::TouchCnt(TouchCntReg::TwlPenDown), 0x38, 5 << 3);
    cdc_write_mask(CdcReg::TouchCnt(TouchCntReg::TwlPenDown), 0x40, 0);
    cdc_write_reg(CdcReg::TouchCnt(TouchCntReg::SarAdcCnt2), 0x8B);

    cdc_write_mask(CdcReg::TouchCnt(TouchCntReg::PanelVoltageStabilization), 0b111, 4);
    cdc_write_mask(CdcReg::TouchCnt(TouchCntReg::PrechargeSense), 0b111, 6);
    cdc_write_mask(CdcReg::TouchCnt(TouchCntReg::PrechargeSense), 0b1110000, 0x40);
    cdc_write_mask(CdcReg::TouchCnt(TouchCntReg::DebouncePenup), 0b111, 0);
    cdc_write_mask(CdcReg::TouchCnt(TouchCntReg::TwlPenDown), 0x80, (1 << 7));
}

pub unsafe fn cdc_write_mask(reg: impl Into<CdcReg>, mask: u8, value: u8) {
    let (bank, reg) = reg.into().as_bank_and_reg();
    bank_switch_tsc(bank);
    let original = read_tsc(reg);
    write_tsc(reg, (original & !mask) | (value & mask));
    super::SPI_HARDWARE.wait_busy();
}
pub unsafe fn is_pen_down() -> bool {
    (cdc_read_reg(TouchCntReg::Status) & 0xC0 != 0x40) //&& (cdc_read_reg(TouchCntReg::TwlPenDown) & 2 == 0)
}
static mut CUR_BANK: u8 = 0x63;
unsafe fn bank_switch_tsc(bank: u8) {
    if bank != CUR_BANK {
        let write = if CUR_BANK == 0xff { 0x7F } else { 0 };
        write_tsc(write, bank);
        CUR_BANK = bank;
    }
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
    let (bank, reg) = start_reg.as_bank_and_reg();
    bank_switch_tsc(bank);
    super::SPI_HARDWARE.wait_busy();
    super::SPI_HARDWARE
        .set_control(SPIControl::ENABLE | SPIControl::DEVICE_CODEC | SPIControl::SELECT_HOLD);
    super::SPI_HARDWARE.write_value(reg << 1 | 1);

    for byte in data {
        *byte = super::SPI_HARDWARE.read_value();
    }
}
unsafe fn write_tsc(reg: u8, value: u8) {
    super::SPI_HARDWARE.wait_busy();
    super::SPI_HARDWARE
        .set_control(SPIControl::ENABLE | SPIControl::DEVICE_CODEC | SPIControl::SELECT_HOLD);
    super::SPI_HARDWARE.write_value(reg << 1);
    super::SPI_HARDWARE.set_control(SPIControl::ENABLE | SPIControl::DEVICE_CODEC);
    super::SPI_HARDWARE.write_value(value);
}
unsafe fn read_tsc(reg: u8) -> u8 {
    super::SPI_HARDWARE.wait_busy();
    super::SPI_HARDWARE
        .set_control(SPIControl::ENABLE | SPIControl::DEVICE_CODEC | SPIControl::SELECT_HOLD);
    super::SPI_HARDWARE.write_value(reg << 1 | 1);
    super::SPI_HARDWARE.set_control(SPIControl::ENABLE | SPIControl::DEVICE_CODEC);
    super::SPI_HARDWARE.read_value()
}

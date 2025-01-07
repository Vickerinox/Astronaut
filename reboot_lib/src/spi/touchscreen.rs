use super::SPIControl;
struct RawTouchData {

}

/* 
pub struct TouchReadError;
unsafe fn touch_read_data() -> Result<RawTouchData, TouchReadError> {
    crate::critical_function(||{
        
    });
}
*/

#[repr(u8)]
pub enum CdcRegister {
    Control(CntReg),     //= 0x00, //< Chip control
    Sound(SndReg),       //= 0x01, //< ADC/DAC control
    TouchCnt(TouchCntReg),//    = 0x03, //< TSC control
    AdcCoefficients(u8),
    BufferModeData(u8),
}
impl CdcRegister {
    pub const fn as_bank_and_reg(self) -> (u8, u8) {
        match self  {
            Self::Control(reg) => (0, reg as u8),
            Self::Sound(reg) => (1, reg as u8),
            Self::TouchCnt(reg) => (3, reg as u8),
            Self::AdcCoefficients(reg) => (4, reg),
            Self::BufferModeData(reg) => (0xFC, reg),
            //CdcRegister::TOUCHDATA => todo!(),
        }
    }
}

impl Into<CdcRegister> for CntReg {
    fn into(self) -> CdcRegister {
        CdcRegister::Control(self)
    }
}
impl Into<CdcRegister> for SndReg {
    fn into(self) -> CdcRegister {
        CdcRegister::Sound(self)
    }
}
impl Into<CdcRegister> for TouchCntReg {
    fn into(self) -> CdcRegister {
        CdcRegister::TouchCnt(self)
    }
}
#[repr(u8)]
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
    
    DebouncePendown = 0x14
}
#[repr(u8)]
pub enum CntReg
{
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
    cdc_write_reg(CntReg::Reset, 1);
    cdc_write_reg(CntReg::NocashAdcDcMeasurement1, 0x66);
    cdc_write_reg(SndReg::ClassDSpeakerAmp, 0x16);
    cdc_write_reg(CntReg::ClockMux, 0);
    cdc_write_reg(CntReg::AdcNadc, 0x81);
    cdc_write_reg(CntReg::AdcMadc, 0x82);
    cdc_write_reg(CntReg::AdcMic, 0x82);
    cdc_write_reg(CntReg::AdcMic, 0);
    cdc_write_reg(CntReg::ClockMux, 3);

    cdc_write_reg(CntReg::PllPr, 0xA1);
    cdc_write_reg(CntReg::PllJ, 0x15);

    cdc_write_reg(CntReg::DacNdac, 0x87);
    cdc_write_reg(CntReg::DacMdac, 0x83);
    cdc_write_reg(CntReg::AdcNadc, 0x87);
    cdc_write_reg(CntReg::AdcMadc, 0x83);

    cdc_write_reg(TouchCntReg::ScanModeTimerClock, 0x88);

    //sound init?
    cdc_write_array(CdcRegister::AdcCoefficients(0x8), &[0x7F,0xE1,0x80,0x1F,0x7F,0xC1]);
    cdc_write_reg(CntReg::DacVolumeLeft, 8);
    cdc_write_reg(CntReg::DacVolumeRight, 8);
    cdc_write_reg(CntReg::GPIO3Pin, 0);
    cdc_write_array(CdcRegister::AdcCoefficients(0x8), &[0x7F,0xE1,0x80,0x1F,0x7F,0xC1]);
    cdc_write_reg(SndReg::MicGain, 0x2B);

    cdc_write_reg(SndReg::FineGain, 0x40);
    cdc_write_reg(SndReg::InputSelection, 0x40);
    cdc_write_reg(SndReg::CmSetting, 0x60);

    cdc_write_reg(CntReg::SarAdc, 0x82);
    cdc_write_reg(CntReg::SarAdc, 0x92);
    cdc_write_reg(CntReg::SarAdc, 0xD2);

    cdc_write_reg(SndReg::PopRemovalSetting, 0x20);
    cdc_write_reg(SndReg::RampDownPeriod, 0xF0);
    cdc_write_reg(CntReg::DacCtrl, 0xD4);
    cdc_write_reg(SndReg::DacMixerRouting, 0x44);
    cdc_write_reg(SndReg::HeadphoneDriver, 0xD4);
    cdc_write_reg(SndReg::DriverHPL, 0x4E);
    cdc_write_reg(SndReg::DriverHPR, 0x4E);

    cdc_write_reg(SndReg::VolumeHPL, 0x9E);
    cdc_write_reg(SndReg::VolumeHPR, 0x9E);

    cdc_write_reg(SndReg::ClassDSpeakerAmp, 0xD4);

    cdc_write_reg(SndReg::DriverSPL, 0x14);
    cdc_write_reg(SndReg::DriverSPR, 0x14);

    cdc_write_reg(SndReg::VolumeSPL, 0xA7);
    cdc_write_reg(SndReg::VolumeSPR, 0xA7);

    cdc_write_reg(CntReg::DacVolume, 0);
    let value = core::ptr::read_volatile(0x4004C00 as *mut u16);
    core::ptr::write_volatile(0x4004C00 as *mut u16, value | 0x80);
    cdc_write_reg(CntReg::GPIO3Pin, 0x60);

    //ENABLE
    cdc_write_reg(TouchCntReg::TwlPenDown, 0);
    cdc_write_reg(TouchCntReg::SarAdcCnt1, 0x18);
    cdc_write_reg(TouchCntReg::ScanModeTimer, 0xA0);
    cdc_write_mask(TouchCntReg::TwlPenDown, 0x38, 0x28);
    cdc_write_mask(TouchCntReg::TwlPenDown, 0x40, 0);
    cdc_write_reg(TouchCntReg::SarAdcCnt2, 0x87);

    cdc_write_mask(TouchCntReg::PanelVoltageStabilization, 0b111, 4);
    cdc_write_mask(TouchCntReg::PrechargeSense, 0b111, 2);
    cdc_write_mask(TouchCntReg::PrechargeSense, 0b1110000, 0x40);
    cdc_write_mask(TouchCntReg::DebouncePenup, 0b111, 0);
    cdc_write_mask(TouchCntReg::TwlPenDown, (1<<7), (1<<7));
}
pub unsafe fn cdc_write_mask(reg: impl Into<CdcRegister>, mask: u8, value: u8) {
    let (bank, reg) = reg.into().as_bank_and_reg();
    bank_switch_tsc(bank);
    let original = read_tsc(reg);
    write_tsc(reg, (original & !mask) | (value & mask));
    super::SPI_HARDWARE.wait_busy();

}
pub unsafe fn is_pen_down() -> bool {
    cdc_read_reg(TouchCntReg::Status) & 0x40 == 0 &&
    cdc_read_reg(TouchCntReg::TwlPenDown) & 3 == 0
}
unsafe fn bank_switch_tsc(bank: u8) {
    write_tsc(0, bank);
}
unsafe fn cdc_read_reg(reg: impl Into<CdcRegister>) -> u8 {
    let (bank, reg) = reg.into().as_bank_and_reg();
    bank_switch_tsc(bank);
    read_tsc(reg)
}
unsafe fn cdc_write_reg(reg: impl Into<CdcRegister>, value: u8) {
    let (bank, reg) = reg.into().as_bank_and_reg();
    bank_switch_tsc(bank);
    write_tsc(reg, value);
    super::SPI_HARDWARE.wait_busy();

}
pub unsafe fn cdc_write_array(start_reg: impl Into<CdcRegister>, data: &[u8]) {
    let (bank, reg) = start_reg.into().as_bank_and_reg();
    bank_switch_tsc(bank);
    super::SPI_HARDWARE.set_control(SPIControl::ENABLE | SPIControl::DEVICE_CODEC | SPIControl::SELECT_HOLD);
    super::SPI_HARDWARE.write_value(reg<<1);
    for byte in data {
        super::SPI_HARDWARE.write_value(*byte);
    }
    super::SPI_HARDWARE.set_control(SPIControl::DISABLE);
}
pub unsafe fn cdc_read_array(start_reg: CdcRegister, data: &mut [u8]) {
    let (bank, reg) = start_reg.as_bank_and_reg();
    bank_switch_tsc(bank);
    super::SPI_HARDWARE.wait_busy();
    super::SPI_HARDWARE.set_control(SPIControl::ENABLE | SPIControl::DEVICE_CODEC | SPIControl::SELECT_HOLD);
    super::SPI_HARDWARE.write_value(reg<<1 | 1);

    for byte in data {
        *byte = super::SPI_HARDWARE.read_value();
    }
}
unsafe fn write_tsc(reg: u8, value: u8) {
    super::SPI_HARDWARE.wait_busy();
    super::SPI_HARDWARE.set_control(SPIControl::ENABLE | SPIControl::DEVICE_CODEC | SPIControl::SELECT_HOLD);
    super::SPI_HARDWARE.write_value(reg<<1);
    super::SPI_HARDWARE.set_control(SPIControl::ENABLE | SPIControl::DEVICE_CODEC);
    super::SPI_HARDWARE.write_value(value);
}
unsafe fn read_tsc(reg: u8) -> u8 {
    super::SPI_HARDWARE.wait_busy();
    super::SPI_HARDWARE.set_control(SPIControl::ENABLE | SPIControl::DEVICE_CODEC | SPIControl::SELECT_HOLD);
    super::SPI_HARDWARE.write_value(reg<<1 | 1);
    super::SPI_HARDWARE.set_control(SPIControl::ENABLE | SPIControl::DEVICE_CODEC);
    super::SPI_HARDWARE.read_value()
}
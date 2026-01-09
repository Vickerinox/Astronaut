use crate::spi::{Control, SPI_HARDWARE, write_powerman};

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
    VolGain = 0x75
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
    let Some(mut current) = iter.next() else { return};
    

    SPI_HARDWARE.set_control(SPIControl::ENABLE | SPIControl::DEVICE_TOUCHSCR | SPIControl::SELECT_HOLD);
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
pub unsafe fn read_tsc_pos() -> Option<(u16, u16)> {
    let mut raw_data = [0u8; 40];
    //let mut raw_data = [0u16; 20];
    crate::critical_function(|| {
        //read_tsc_var(0x85 | 0x30, &mut raw_data[0..5]);
        //read_tsc_var(0x85 | 0x40, &mut raw_data[5..10]);
        //read_tsc_var(0x85 | 0x50, &mut raw_data[10..15]);
        //read_tsc_var(0x85 | 0x10, &mut raw_data[15..20]);
        cdc_read_array(CdcReg::BufferModeData(1), &mut raw_data);
    });
    let mut rawx = 0;
    let mut rawy = 0;

    for i in (0..10).step_by(2) {
        let x = u16::from_le_bytes([raw_data[i+1], raw_data[i]]);
        let y = u16::from_le_bytes([raw_data[i+11], raw_data[i+10]]);
        //if (x | y) & 0xF000 > 0 { return None }
        rawx += x & 0xFFF; //raw_data[10+i];
        rawy += y & 0xFFF; //raw_data[15+i];
    }
    Some((rawx/5, rawy/5)) 
}
pub unsafe fn init_tsc() {
    //GPIO pin 3 enable
    core::ptr::write_volatile(0x4004C00 as *mut u16, 0x8080);
    //write_powerman(crate::spi::PowerRegiser::Control(Control::ENABLE_SOUND_AMP | Control::ENABLE_BACKLIGHTS));
    cdc_write_reg(CdcReg::Control(CntReg::Reset), 1);
    crate::swi_delay(0x20BA); //Wait 1ms for reset (recommended by TSC2117 technical sheet, but never done by anyone else?)

    //cdc_read_reg(CdcReg::UndocumentedReset);
    
    
    /* 
    cdc_read_reg(CdcReg::Control(CntReg::AdcMic));
    cdc_read_reg(CdcReg::TouchCnt(TouchCntReg::SarAdcCnt1));
    cdc_read_reg(CdcReg::Control(CntReg::DacCtrl));
    cdc_read_reg(CdcReg::Sound(SndReg::DriverHPL));
    cdc_read_reg(CdcReg::Sound(SndReg::DriverSPL));
    cdc_read_reg(CdcReg::Sound(SndReg::MicBias));
    */

    const INIT_LIST: &[(CdcReg, u8)] = &[
        //Pre-init
        (CdcReg::TouchCnt(TouchCntReg::SarAdcCnt1), 0x80),
        /* 
        (CdcReg::Control(CntReg::GPIO3Pin), 0),
        (CdcReg::Control(CntReg::AdcVolFine), 0x80),
        (CdcReg::Control(CntReg::DacVolume), 0xC),
        (CdcReg::Sound(SndReg::VolumeHPL), 0xFF),
        (CdcReg::Sound(SndReg::VolumeHPR), 0xFF),
        (CdcReg::Sound(SndReg::VolumeSPL), 0x7F),
        (CdcReg::Sound(SndReg::VolumeSPR), 0x7F),
        (CdcReg::Sound(SndReg::DriverHPL), 0x4A),
        (CdcReg::Sound(SndReg::DriverHPR), 0x4A),
        (CdcReg::Sound(SndReg::DriverSPL), 0x10),
        (CdcReg::Sound(SndReg::DriverSPR), 0x10),
        (CdcReg::Control(CntReg::AdcMic), 0x0),
        
        (CdcReg::Sound(SndReg::DacMixerRouting), 0x0),
        (CdcReg::Sound(SndReg::HeadphoneDriver), 0x14),
        (CdcReg::Sound(SndReg::ClassDSpeakerAmp), 0x14),
        (CdcReg::Control(CntReg::DacCtrl), 0x0),
        (CdcReg::Control(CntReg::PllPr), 0x0),
        (CdcReg::Control(CntReg::DacNdac), 0x1),
        (CdcReg::Control(CntReg::DacMdac), 0x2),
        (CdcReg::Control(CntReg::AdcNadc), 0x1),
        (CdcReg::Control(CntReg::AdcMadc), 0x2),
        (CdcReg::Sound(SndReg::MicBias), 0x0),
        (CdcReg::Control(CntReg::GPIO3Pin), 0x60),
        */
        //touchscreen and sound amp init
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
        (CdcReg::Control(CntReg::VolSarAdc), 0xD2),         // 62HZ throughput + DAC ON + 1bit hysterisis  
        (CdcReg::Sound(SndReg::PopRemovalSetting), 0x20), //15MS pop removal (awesome)
        (CdcReg::Sound(SndReg::RampDownPeriod), 0xF0),
        (CdcReg::Control(CntReg::DacCtrl), 0xD4),       // Power up DACs + route R->R + route L->L
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

        
        (CdcReg::Control(CntReg::DacVolume), 0),   //unmute L/R DACs and let them have independent control
        (CdcReg::Control(CntReg::GPIO3Pin), 0x60), //enable + reserved bit?
        
        
        /* 
         //Put into NDS mode
        (CdcReg::Sound(SndReg::VolumeSPL), 0xA7),
        (CdcReg::Sound(SndReg::VolumeSPR), 0xA7),
        (CdcReg::Sound(SndReg::MicBias), 0x3),
        (CdcReg::TouchCnt(TouchCntReg::SarAdcCnt2), 0),
        (CdcReg::Sound(SndReg::PopRemovalSetting), 0x20),
        (CdcReg::Sound(SndReg::RampDownPeriod), 0xF0),
        (CdcReg::Sound(SndReg::RampDownPeriod), 0x70),
        (CdcReg::Control(CntReg::AdcVolFine), 0x80),
        (CdcReg::Control(CntReg::AdcMic), 0x00),


        //post-init (fill non-populated regs with default values)
        
        (CdcReg::Control(CntReg::OverTemp), 0x44), //RO?
        (CdcReg::Control(CntReg::DOSRMSB), 0x00),
        (CdcReg::Control(CntReg::DOSRLSB), 0x80),
        (CdcReg::Control(CntReg::IDAC), 0x80),
        (CdcReg::Control(CntReg::Interpolation), 0x08),
        (CdcReg::Control(CntReg::AOSR), 0x80),
        (CdcReg::Control(CntReg::IADC), 0x80),
        (CdcReg::Control(CntReg::Decimation), 0x04),
        (CdcReg::Control(CntReg::ClkDivM), 0x01),
        (CdcReg::Control(CntReg::ClkDivN), 0x01),
        (CdcReg::Control(CntReg::AdcFlags), 0x80),
        (CdcReg::Control(CntReg::GPIO1Control), 0x34),
        (CdcReg::Control(CntReg::GPIO2Control), 0x32),
        (CdcReg::Control(CntReg::SdIn), 0x12),
        (CdcReg::Control(CntReg::SdOut), 0x03),
        (CdcReg::Control(CntReg::MISO), 0x02),
        (CdcReg::Control(CntReg::SCLK), 0x03),
        (CdcReg::Control(CntReg::DacInstructionSet), 0x19),
        (CdcReg::Control(CntReg::AdcInstructionSet), 0x05),
        (CdcReg::Control(CntReg::DacControl1), 0x0F),
        (CdcReg::Control(CntReg::DacControl2), 0x38),
        (CdcReg::Control(CntReg::DacBeepLen1), 0x00),
        (CdcReg::Control(CntReg::DacBeepLen2), 0x00),
        (CdcReg::Control(CntReg::DacBeepLen3), 0xEE),
        (CdcReg::Control(CntReg::DacBeepSinMSB), 0x10),
        (CdcReg::Control(CntReg::DacBeepSinLSB), 0xD8),
        (CdcReg::Control(CntReg::DacBeepCosMSB), 0x7E),
        (CdcReg::Control(CntReg::DacBeepCosMSB), 0xE3),
        (CdcReg::Control(CntReg::AGCMaxGain), 0x7F),
        (CdcReg::Control(CntReg::VolSarAdc), 0xD2),
        (CdcReg::Control(CntReg::VolGain), 0x2C),

        (CdcReg::Sound(SndReg::RampDownPeriod), 0x70),
        (CdcReg::Sound(SndReg::DriverCnt), 0x20),
        

        (CdcReg::TouchCnt(TouchCntReg::SarAdcCnt1), 0x98),
        (CdcReg::TSCNDSMode, 0)
        */
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
    //cdc_write_reg(CdcReg::Control(CntReg::DacVolume), 0);
    //moved...
    //cdc_write_reg(CdcReg::Control(CntReg::GPIO3Pin), 0x60);
}
pub unsafe fn enable_tsc() {
    /* 
    cdc_write_reg(CdcReg::Sound(SndReg::MicBias), 0x03);
    cdc_write_reg(CdcReg::Control(CntReg::AdcMic), 0x80);
    cdc_write_reg(CdcReg::Control(CntReg::AdcVolFine), 0x0);
    cdc_write_reg(CdcReg::Sound(SndReg::MicGain), 0x037);
    */
    //ENABLE?
    cdc_write_mask(CdcReg::TouchCnt(TouchCntReg::TwlBufferMode), 0x80, 0);
    cdc_write_reg(CdcReg::TouchCnt(TouchCntReg::SarAdcCnt1), 0x18);

    cdc_write_reg(CdcReg::TouchCnt(TouchCntReg::ScanModeTimer), 0xA0);
    cdc_write_mask(CdcReg::TouchCnt(TouchCntReg::TwlBufferMode), 0x38, 5 << 3);
    cdc_write_mask(CdcReg::TouchCnt(TouchCntReg::TwlBufferMode), 0x40, 0);
    cdc_write_reg(CdcReg::TouchCnt(TouchCntReg::SarAdcCnt2), 0x88);

    cdc_write_mask(CdcReg::TouchCnt(TouchCntReg::PanelVoltageStabilization), 0b111, 4);
    cdc_write_mask(CdcReg::TouchCnt(TouchCntReg::PrechargeSense), 0b1110111, 0x66);
    cdc_write_mask(CdcReg::TouchCnt(TouchCntReg::DebouncePenup), 0b111, 0);
    cdc_write_reg(CdcReg::TouchCnt(TouchCntReg::TwlBufferMode), 0x80 | (5 << 3));
    cdc_write_reg(CdcReg::TouchCnt(TouchCntReg::SarAdcCnt1), 0x18);
    //cdc_write_reg(CdcReg::TouchCnt(TouchCntReg::BufferMode), 0x80 | (5 << 3));
}

pub unsafe fn cdc_write_mask(reg: impl Into<CdcReg>, mask: u8, value: u8) {
    let (bank, reg) = reg.into().as_bank_and_reg();
    bank_switch_tsc(bank);
    let original = read_tsc(reg);
    write_tsc(reg, (original & !mask) | (value & mask));
    super::SPI_HARDWARE.wait_busy();
}
pub unsafe fn is_pen_down() -> bool {
    (cdc_read_reg(TouchCntReg::Status) & 0x80 > 0) && (cdc_read_reg(TouchCntReg::TwlBufferMode) & 2 == 0)
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
    let Some(mut current) = iter.next() else { return};
    
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
    super::SPI_HARDWARE
        .set_control(SPIControl::ENABLE | SPIControl::DEVICE_CODEC);
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

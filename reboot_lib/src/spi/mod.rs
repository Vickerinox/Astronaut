use volatile_register::RW;

use crate::MemoryWrapper;

pub mod firmware;
pub mod touchscreen;
pub mod tsc2117;
///SPI bus functions

pub const SPI_HARDWARE: MemoryWrapper<SerialPeripheralInterface> =
    MemoryWrapper(0x40001C0 as *mut SerialPeripheralInterface);
#[repr(C)]
pub struct SerialPeripheralInterface {
    control_and_status: RW<SPIControl>,
    data: RW<u16>,
}
bitflags::bitflags! {
    #[derive(Clone, Copy)]
    pub struct SPIControl: u16 {
        const DEVICE_POWERMAN = (0<<8) | 2;
        const DEVICE_FIRMWARE = (1<<8) | 0;
        const DEVICE_TOUCHSCR = (2<<8) | 1;
        const DEVICE_CODEC = (2<<8) | 0;
        const SELECT_HOLD = (1<<11);
        const ENABLE = (1<<15);
        const BUSY = (1<<7);
        const DISABLE = 0;
    }
}

impl SerialPeripheralInterface {
    //loop until spi is not busy

    #[inline(always)]
    fn wait_busy(&self) {
        while self.control_and_status.read().contains(SPIControl::BUSY) {}
    }
    unsafe fn set_control(&self, control: SPIControl) {
        self.control_and_status.write(control);
    }
    unsafe fn write_value(&self, value: u8) {
        self.data.write(value as u16);
        self.wait_busy();
    }
    unsafe fn exchange_raw_value(&self, value: u8) -> u8 {
        self.write_value(value);
        self.wait_busy();
        self.data.read() as u8
    }
    unsafe fn read_value(&self) -> u8 {
        self.exchange_raw_value(0)
    }
    unsafe fn bank_switch_tsc(bank: u8) {}
    pub unsafe fn write_pm(&self, register: PowerRegiser) -> u8 {
        let mut ret = 0;
        crate::critical_function(|| {
            let (reg, val) = register.as_reg_and_value();
            self.wait_busy();
            self.control_and_status
                .write(SPIControl::ENABLE | SPIControl::DEVICE_POWERMAN | SPIControl::SELECT_HOLD);
            self.write_value(reg);
            self.control_and_status
                .write(SPIControl::ENABLE | SPIControl::DEVICE_POWERMAN);
            ret = self.exchange_raw_value(val);
        });
        ret
    }
    pub unsafe fn read_firmware(&self, buffer: &mut [u8], start: u32) {
        crate::critical_function(|| {
            self.wait_busy();
            self.control_and_status
                .write(SPIControl::ENABLE | SPIControl::DEVICE_FIRMWARE | SPIControl::SELECT_HOLD);
            self.wait_busy();
            self.write_value(0x3);
            let [a,b,c,d] = start.to_le_bytes();
            self.write_value(c);
            self.write_value(b);
            self.write_value(a);
            for byte in buffer {
                *byte = self.read_value();
            }
            self.control_and_status.write(SPIControl::DISABLE);
        });
    }
}

pub enum PowerRegiser {
    Control(Control),
    BatteryStatus(BatteryStatus),
    MicrophoneAmp(MicrophoneAmp),
    MicrophoneGain(MicrophoneGain),
    Backlight(Backlight),
    Reset(Reset),
    ReadRegister,
}
bitflags::bitflags! {
    pub struct Control: u8 {
        const ENABLE_SOUND_AMP = (1 << 0);
        const MUTE_SOUND_AMP = (1 << 1);
        const ENABLE_LOWER_BACKLIGHT = (1 << 2);
        const ENABLE_UPPER_BACKLIGHT = (1 << 3);
        const ENABLE_BACKLIGHTS = (0b11 << 2);
        const POWER_LED_BLINK = (1 << 4);
        const POWER_LED_SPEED = (1 << 5);
        const SHUT_DOWN_POWER = (1 << 6);
    }
    pub struct BatteryStatus: u8 {
        const POWER_IS_LOW = (1 << 0);
    }
    pub struct MicrophoneAmp: u8 {
        const ENABLE = (1 << 0);
    }
    pub struct Backlight: u8 {
        const BACKLIGHT_LOW = (0 << 0);
        const BACKLIGHT_MED = (1 << 0);
        const BACKLIGHT_HIGH = (2 << 0);
        const BACKLIGHT_MAX = (3 << 0);
        const FORCE_MAX_WHEN_CHARGING = (1<<2);
        const IS_CHARGING = (1<<3);
    }
    pub struct Reset: u8 {
        const RESET = (1<<0);
    }

}
impl PowerRegiser {
    fn as_reg_and_value(self) -> (u8, u8) {
        match self {
            PowerRegiser::Control(control) => (0, control.bits()),
            PowerRegiser::BatteryStatus(battery_status) => (1, battery_status.bits()),
            PowerRegiser::MicrophoneAmp(microphone_amp) => (2, microphone_amp.bits()),
            PowerRegiser::MicrophoneGain(microphone_gain) => (3, microphone_gain as u8),
            PowerRegiser::Backlight(backlight) => (4, backlight.bits()),
            PowerRegiser::Reset(reset) => (0x10, reset.bits()),
            PowerRegiser::ReadRegister => (0x80, 0),
        }
    }
}
#[repr(u8)]
pub enum MicrophoneGain {
    Low = 0,
    Med = 1,
    High = 2,
    Max = 3,
}

pub unsafe fn write_powerman(reg: PowerRegiser) {
    let (reg, value) = reg.as_reg_and_value();
    crate::critical_function(|| {
        SPI_HARDWARE.wait_busy();
        SPI_HARDWARE
            .control_and_status
            .write(SPIControl::ENABLE | SPIControl::DEVICE_POWERMAN | SPIControl::SELECT_HOLD);
        SPI_HARDWARE.write_value(reg);
        SPI_HARDWARE
            .control_and_status
            .write(SPIControl::ENABLE | SPIControl::DEVICE_POWERMAN);
        SPI_HARDWARE.exchange_raw_value(value);
        SPI_HARDWARE.control_and_status.write(SPIControl::DISABLE);
    });
}

unsafe fn write_tsc(reg: u8, value: u8) {
    SPI_HARDWARE.wait_busy();
    SPI_HARDWARE
        .control_and_status
        .write(SPIControl::ENABLE | SPIControl::DEVICE_CODEC | SPIControl::SELECT_HOLD);
    SPI_HARDWARE.write_value(reg << 1);
    SPI_HARDWARE
        .control_and_status
        .write(SPIControl::ENABLE | SPIControl::DEVICE_CODEC);
    SPI_HARDWARE.write_value(value);
}
static mut CUR_BANK: u8 = 0x63;
unsafe fn tsc_switch_bank(bank: u8) {
    if bank != CUR_BANK {
        let reg = match CUR_BANK {
            0xFF => 0x7f,
            _ => 0,
        };
        write_tsc(reg, bank);
        CUR_BANK = bank
    }
}

fn touch_init() {}
fn cdc_write_reg(bank: u8, reg: u8, value: u8) {}
fn cdc_write_reg_mask(bank: u8, reg: u8, mask: u8, value: u8) {}

//master interrupt enable register.
unsafe fn read_firmware(offset: usize, buffer: &mut [u8]) {
    crate::critical_function(|| {
        SPI_HARDWARE.set_control(
            SPIControl::SELECT_HOLD | SPIControl::DEVICE_FIRMWARE | SPIControl::ENABLE,
        );
        SPI_HARDWARE.write_value(3);
        SPI_HARDWARE.write_value((offset >> 16) as u8);
        SPI_HARDWARE.write_value((offset >> 8) as u8);
        SPI_HARDWARE.write_value(offset as u8);
        for byte in buffer.iter_mut() {
            *byte = SPI_HARDWARE.read_value();
        }
        SPI_HARDWARE.set_control(SPIControl::empty());
    });
}

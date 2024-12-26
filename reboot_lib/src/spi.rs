use volatile_register::RW;

use crate::RegisterWrapper;

///SPI bus functions

const SPI_HARDWARE: RegisterWrapper<SerialPeripheralInterface> =
    RegisterWrapper(0x40001C0 as *mut SerialPeripheralInterface);

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
        const ENABLE_BUS = (1<<15);
        const BUSY = (1<<7);
    }
}

impl SerialPeripheralInterface {
    //loop until spi is not busy
    fn wait_busy(&self) {
        while self.control_and_status.read().contains(SPIControl::BUSY) {}
    }
    unsafe fn set_control(&self, control: SPIControl) {
        self.control_and_status.write(control);
    }
    unsafe fn write_raw_value(&self, value: u8) {
        self.data.write(value as u16);
        self.wait_busy();
    }
    unsafe fn exchange_raw_value(&self, value: u8) -> u8 {
        self.write_raw_value(value);
        self.data.read() as u8
    }
    unsafe fn read_raw_value(&self) -> u8 {
        self.exchange_raw_value(0)
    }
}

pub unsafe fn write_powerman(reg: u8, value: u8) {
    SPI_HARDWARE.wait_busy();
    SPI_HARDWARE
        .control_and_status
        .write(SPIControl::ENABLE_BUS | SPIControl::DEVICE_POWERMAN | SPIControl::SELECT_HOLD);
    SPI_HARDWARE.write_raw_value(reg);
    SPI_HARDWARE
        .control_and_status
        .write(SPIControl::ENABLE_BUS | SPIControl::DEVICE_POWERMAN);
    SPI_HARDWARE.write_raw_value(value);
}


unsafe fn write_tsc(reg: u8, value: u8) {
    SPI_HARDWARE.wait_busy();
    SPI_HARDWARE
        .control_and_status
        .write(SPIControl::ENABLE_BUS | SPIControl::DEVICE_CODEC | SPIControl::SELECT_HOLD);
    SPI_HARDWARE.write_raw_value(reg << 1);
    SPI_HARDWARE
        .control_and_status
        .write(SPIControl::ENABLE_BUS | SPIControl::DEVICE_CODEC);
    SPI_HARDWARE.write_raw_value(value);
}
static mut cur_bank: u8 = 0x63;
unsafe fn tsc_switch_bank(bank: u8) {
    if bank != cur_bank {
        let reg = match cur_bank {
            0xFF => 0x7f,
            _ => 0,
        };
        write_tsc(reg, bank);
        cur_bank = bank
    }
}

fn touch_init() {}
fn cdc_write_reg(bank: u8, reg: u8, value: u8) {}
fn cdc_write_reg_mask(bank: u8, reg: u8, mask: u8, value: u8) {}

//master interrupt enable register.
unsafe fn read_firmware(offset: usize, buffer: &mut [u8]) {
    crate::critical_function(|| {
        SPI_HARDWARE.set_control(
            SPIControl::SELECT_HOLD | SPIControl::DEVICE_FIRMWARE | SPIControl::ENABLE_BUS,
        );
        SPI_HARDWARE.write_raw_value(3);
        SPI_HARDWARE.write_raw_value((offset >> 16) as u8);
        SPI_HARDWARE.write_raw_value((offset >> 8) as u8);
        SPI_HARDWARE.write_raw_value(offset as u8);
        for byte in buffer.iter_mut() {
            *byte = SPI_HARDWARE.read_raw_value();
        }
        SPI_HARDWARE.set_control(SPIControl::empty());
    });
}

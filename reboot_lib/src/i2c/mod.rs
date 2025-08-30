use crate::MemoryWrapper;
use bitflags::bitflags;
use volatile_register::RW;
pub const I2C_HARDWARE: MemoryWrapper<I2CInterface> = MemoryWrapper(0x4004500 as *mut I2CInterface);

pub unsafe fn init() {
    I2C_HARDWARE.write_register(PowerRegister::WIFILED, 1);
    I2C_HARDWARE.write_register(PowerRegister::MMCPWR, 1);
    //I2C_HARDWARE.write_register(PowerRegister::PowerButtonTap, 0x10);
    //I2C_HARDWARE.write_register(PowerRegister::PowerButtonHold, 0x64);
}
#[repr(C)]
pub struct I2CInterface {
    data: RW<u8>,
    control: RW<u8>,
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct I2CControl: u8 {
        const STOP = (1<<0);
        const START = (1<<1);
        const ERROR = (1<<2);

        const ACK = (1<<4);
        const DATA_DIRECTION = (1<<5);
        const ENABLE_INTERRUPT = (1<<6);
        const START_BUSY = (1<<7);
    }
}
impl I2CInterface {
    unsafe fn okay(&self) -> bool {
        self.wait_busy();
        self.control.read() & 0x10 > 0
    }
    unsafe fn wait_busy(&self) {
        while self.control.read() & 0x80 > 0 {}
    }
    unsafe fn set_device(&self, device: u8) -> Result<I2CSuccess, I2CFailure> {
        self.wait_busy();
        crate::swi_delay(0x180);
        self.data.write(device);
        self.control.write((1 << 7) | (1 << 1) | (1 << 6));
        self.get_result()
    }
    unsafe fn set_register(&self, register: u8) -> Result<I2CSuccess, I2CFailure> {
        self.wait_busy();
        crate::swi_delay(0x180);
        self.data.write(register);
        self.control.write((1 << 7) | (1 << 6));
        self.get_result()
    }
    pub unsafe fn write_register(
        &self,
        register: impl Into<I2CRegister>,
        value: u8,
    ) -> Result<I2CSuccess, I2CFailure> {
        let (device, register) = register.into().as_chip_and_reg();
        for i in 0..8 {
            if self.set_device(device).is_ok() && self.set_register(register).is_ok() {
                crate::swi_delay(0x180);
                self.data.write(value);
                self.stop(0);
                if self.get_result().is_ok() {
                    return Ok(I2CSuccess);
                }
            }
            self.control.write((1 << 7) | (1 << 2) | 1 | (1 << 6));
        }
        Err(I2CFailure)
    }
    pub unsafe fn read_register(&self, register: impl Into<I2CRegister>) -> Result<u8, I2CFailure> {
        let (device, register) = register.into().as_chip_and_reg();
        for i in 0..8 {
            if self.set_device(device).is_ok() && self.set_register(register).is_ok() {
                crate::swi_delay(0x180);
                if self.set_device(device | 1).is_ok() {
                    crate::swi_delay(0x180);
                    self.stop((1 << 5));
                    self.wait_busy();
                    return Ok(self.data.read());
                }
            }
            self.control.write((1 << 7) | (1 << 2) | 1 | (1 << 6));
        }
        Err(I2CFailure)
    }

    unsafe fn get_result(&self) -> Result<I2CSuccess, I2CFailure> {
        match self.control.read() & 0x10 > 0 {
            true => Ok(I2CSuccess),
            false => Err(I2CFailure),
        }
    }
    unsafe fn stop(&self, arg: u8) {
        self.control.write(arg | (1 << 7) | (1 << 6));
        self.wait_busy();
        crate::swi_delay(0x180);
        self.control
            .write((1 << 7) | (1 << 2) | (1 << 0) | (1 << 6));
    }
}

pub enum I2CRegister {
    I2cCam0,
    I2cCam1,
    I2cUnk1,
    I2cUnk2,
    I2cPower(PowerRegister),
    I2cUnk3,
    I2cGpio,
}
impl I2CRegister {
    pub fn as_chip_and_reg(self) -> (u8, u8) {
        match self {
            I2CRegister::I2cCam0 => (0x7A, 0),
            I2CRegister::I2cCam1 => (0x78, 0),
            I2CRegister::I2cUnk1 => (0xA0, 0),
            I2CRegister::I2cUnk2 => (0xE0, 0),
            I2CRegister::I2cPower(power_register) => (0x4A, power_register as u8),
            I2CRegister::I2cUnk3 => (0x40, 0),
            I2CRegister::I2cGpio => (0x90, 0),
        }
    }
}
#[repr(u8)]
pub enum PowerRegister {
    BATUNK = 0x00,
    PWRIF = 0x10,
    PWRCNT = 0x11,
    MMCPWR = 0x12,
    BATTERY = 0x20,
    WIFILED = 0x30,
    CAMLED = 0x31,
    VOL = 0x40,
    BACKLIGHT = 0x41,
    RESETFLAG = 0x70,

    PowerButtonTap = 0x80,
    PowerButtonHold = 0x81,
}
impl Into<I2CRegister> for PowerRegister {
    fn into(self) -> I2CRegister {
        I2CRegister::I2cPower(self)
    }
}
pub struct I2CSuccess;
#[derive(Debug)]
pub struct I2CFailure;

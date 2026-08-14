// SPDX-FileCopyrightText: 2026 Viktor Karlsson <viktor@koda.re>
// SPDX-License-Identifier: MIT

use crate::MemoryWrapper;
use bitflags::bitflags;
use volatile_register::RW;

/// Memory wrapper around the registers of the I2C hardware
#[cfg(feature = "arm7i")]
pub const I2C_HARDWARE: MemoryWrapper<I2CInterface> = MemoryWrapper(0x4004500 as *mut I2CInterface);

/// Perform basic initialization of the I2C hardware
#[cfg(feature = "arm7i")]
pub unsafe fn init() {
    I2C_HARDWARE.write_register(PowerRegister::MMCPWR.into(), 0);

    //I2C_HARDWARE.write_register(PowerRegister::PowerButtonTap, 0x10);
    //I2C_HARDWARE.write_register(PowerRegister::PowerButtonHold, 0x64);
}

/// Memory Layout of the I2C hardware registers
#[repr(C)]
pub struct I2CInterface {
    data: RW<u8>,
    control: RW<u8>,
}

bitflags! {
    /// Bitflags for the Control byte of the I2C Interface
    #[derive(Debug, Clone, Copy)]
    pub struct I2CControl: u8 {
        /// Stop transation
        const STOP = (1<<0);
        /// Start transaction
        const START = (1<<1);
        /// Transaction Error
        const ERROR = (1<<2);
        /// Acknowledge action (opposite of data direction)
        const ACK = (1<<4);
        /// Read data
        const DATA_READ = (1<<5);
        /// Write data
        const DATA_WRITE = (0<<5);
        /// Enable I2C interrupt, caused by mainly button presses.
        const ENABLE_INTERRUPT = (1<<6);
        /// Hardware enable (active/busy flag)
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
    /// Write a new value to a device register on the I2C bus.
    pub unsafe fn write_register(
        &self,
        register: I2CRegister,
        value: u8,
    ) -> Result<I2CSuccess, I2CFailure> {
        let (device, register) = register.as_chip_and_reg();
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

    /// Write the value of a device register on the I2C bus.
    pub unsafe fn read_register(&self, register: I2CRegister) -> Result<u8, I2CFailure> {
        let (device, register) = register.as_chip_and_reg();
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

/// An enum over all available I2C registers on the DSi
pub enum I2CRegister {
    /// Interface 0 of the camera module
    I2cCam0,
    /// Interface 1 of the camera module
    I2cCam1,
    /// Unknown interface, theorized to be related to unused camera module
    I2cUnkA0,
    /// Unknown interface, theorized to be related to unused camera module
    I2cUnkE0,
    /// BPTWL chip registers
    I2cPower(PowerRegister),
    /// Unknown interface, theorized to be dev console related.
    I2cUnk40,
    /// GPIO interface
    I2cGpio,
}
impl I2CRegister {
    /// Return the Chip number and register number from an [`I2CRegister`]
    pub fn as_chip_and_reg(self) -> (u8, u8) {
        match self {
            I2CRegister::I2cCam0 => (0x7A, 0),
            I2CRegister::I2cCam1 => (0x78, 0),
            I2CRegister::I2cUnkA0 => (0xA0, 0),
            I2CRegister::I2cUnkE0 => (0xE0, 0),
            I2CRegister::I2cPower(power_register) => (0x4A, power_register as u8),
            I2CRegister::I2cUnk40 => (0x40, 0),
            I2CRegister::I2cGpio => (0x90, 0),
        }
    }
}
/// Register selection on the BPTWL chip
#[repr(u8)]
pub enum PowerRegister {
    /// Revision number (R)
    BATUNK = 0x00,

    /// Power button status
    PWRIF = 0x10,

    /// Force system reset
    PWRCNT = 0x11,

    /// Power button behavior
    MMCPWR = 0x12,

    /// Battery charge state
    BATTERY = 0x20,

    /// Wifi LED behavior
    WIFILED = 0x30,

    /// Camera LED enable
    CAMLED = 0x31,

    /// Volume level
    VOL = 0x40,

    /// Backlight strength level
    BACKLIGHT = 0x41,

    /// Cold/Warm boot indicator (1 = Warmboot)
    RESETFLAG = 0x70,

    /// Power button tap delay
    PowerButtonTap = 0x80,

    /// Power button hold delay
    PowerButtonHold = 0x81,
}
impl Into<I2CRegister> for PowerRegister {
    fn into(self) -> I2CRegister {
        I2CRegister::I2cPower(self)
    }
}
/// Unit type to describe successful I2C transactions
pub struct I2CSuccess;
/// Unit type to describe erroneous I2C transactions
#[derive(Debug)]
pub struct I2CFailure;

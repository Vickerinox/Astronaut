use volatile_register::RW;

use crate::{ClockCnt, Control, DataControl32, SDIO_CONTROLLER, Status, TMIOPort, i2c::PowerRegister};

pub unsafe fn init_nwifi() {
    SDIO_CONTROLLER.soft_reset.modify(|f| f & !3);
    SDIO_CONTROLLER.soft_reset.modify(|f| f | 3);
    SDIO_CONTROLLER.stop_action.modify(|f| (f |0x100) & !1);
    SDIO_CONTROLLER.port_select.write(0);
    SDIO_CONTROLLER.options.write(0x80D0);
    SDIO_CONTROLLER.clock_control.write(ClockCnt::FREQ_131K);
    SDIO_CONTROLLER.options.modify(|i| i|0x100);
    SDIO_CONTROLLER.options.modify(|i| i& !0x100);
    SDIO_CONTROLLER.clock_control.write(ClockCnt::ENABLE);
    SDIO_CONTROLLER.data_control.write(Control::USE_DATA32);
    SDIO_CONTROLLER.data_control_32.write(DataControl32::USE_DATA32 | DataControl32::CLEAR_FIFO_32);
    (*(0x4004C04 as *mut RW<u16>)).modify(|i| i & !0x100);
    crate::i2c::I2C_HARDWARE.write_register(PowerRegister::WIFILED, 0x13);
}
pub unsafe fn init_nwifi_opcond() {
    if SDIO_CONTROLLER.status.read().contains(Status::ERR_CMD_TIMEOUT) {
        let mut test = 0;
        loop {
            test &= 0x100000;
        }
    }
}
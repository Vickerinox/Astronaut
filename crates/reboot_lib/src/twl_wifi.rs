use volatile_register::RW;

use crate::{
    i2c::{PowerRegister, I2C_HARDWARE},
    rtc::RTC_HARDWARE,
    ClockCnt, Control, DataControl32, Status, StorageSector, TMIOPort, SDIO_CONTROLLER,
};

pub struct SDIOPort {
    address: u16,
    clk_cnt: u32,
    bus_width: u16,
    size: u32,
    response: [u32; 4],
    status: u32,
    errors: u32,
    block_size: u16,
    buffer: *mut [StorageSector],
}
impl SDIOPort {}

pub struct Command(u16);

impl Command {}

pub struct CommandComposition {
    kind: CommandType,
    response: ResponseType,
    transfer: Transfer,
    direction: DataDirection,
    length: DataLength,
    security: DataSecurity,
}
impl CommandComposition {
    const DEFAULT: Self = Self {
        kind: todo!(),
        response: todo!(),
        transfer: todo!(),
        direction: todo!(),
        length: todo!(),
        security: todo!(),
    };
}
pub enum CommandType {}
pub enum ResponseType {
    None = 3,
    Normal48Bit = 4,
    Busy48Bit = 5,
    Normal136Bit = 6,
    Ocr48Bit = 7,
}
pub enum Transfer {
    None = 0,
    Some = 1,
}
pub enum DataDirection {
    Write = 0,
    Read = 1,
}
pub enum DataLength {
    SingleBlock = 0,
    MultiBlock = 1,
}
pub enum DataSecurity {
    Unsecure = 0,
    Secure = 1,
}

pub unsafe fn new_nwifi_init() {
    let mut port = TMIOPort::dsio();
    (*(0x4004C04 as *mut RW<u16>)).modify(|i| i & !0x100);
    crate::swi_delay(5 * 134056);

    RTC_HARDWARE.transact(&[0x72u8, 0x80], &mut []);
    RTC_HARDWARE.transact(&[0x74u8, 0x00], &mut []);
    I2C_HARDWARE.write_register(PowerRegister::WIFILED, 0x13);
    (0x4004020 as *mut u16).write_volatile(1);

    {
        SDIO_CONTROLLER.port_select.write(port.port_num);
        SDIO_CONTROLLER.clock_control.write(port.clock);
        SDIO_CONTROLLER.options.write(0);
    }
    crate::swi_delay(0xF000);
    {
        SDIO_CONTROLLER.stop_action.write(0x100);

        let mut ocr = 0;
        port.response[0] = 0;
        while port.response[0] & 0x8000_0000 == 0 {
            while SDIO_CONTROLLER
                .send_command(&mut port, crate::mmc::Command::SDIOOpCond, ocr)
                .contains(Status::ERR_CMD_TIMEOUT)
            {}
            ocr = port.response[0] & 0x100000;
        }
    }

    {
        if !SDIO_CONTROLLER
            .send_command(&mut port, crate::mmc::Command::SetSendRelativeAddr, 0)
            .successful()
        {
            return;
        }
    }
}

pub unsafe fn init_nwifi_regs() {
    SDIO_CONTROLLER.soft_reset.modify(|f| f & !3);
    SDIO_CONTROLLER.soft_reset.modify(|f| f | 3);
    SDIO_CONTROLLER.stop_action.modify(|f| (f | 0x100) & !1);
    SDIO_CONTROLLER.port_select.write(0);
    SDIO_CONTROLLER.options.write(0x80D0);
    SDIO_CONTROLLER.clock_control.write(ClockCnt::FREQ_131K);
    SDIO_CONTROLLER.options.modify(|i| i | 0x100);
    SDIO_CONTROLLER.options.modify(|i| i & !0x100);
    SDIO_CONTROLLER.clock_control.write(ClockCnt::ENABLE);
    SDIO_CONTROLLER.data_control.write(Control::USE_DATA32);
    SDIO_CONTROLLER
        .data_control_32
        .write(DataControl32::USE_DATA32 | DataControl32::CLEAR_FIFO_32);
    SDIO_CONTROLLER.irmask.write(Status::all());
    (*(0x4004C04 as *mut RW<u16>)).modify(|i| i & !0x100);
    crate::i2c::I2C_HARDWARE.write_register(PowerRegister::WIFILED, 0x13);
}
pub unsafe fn init_nwifi_opcond() -> bool {
    nwifi_read_func0(FuncReg::func0(4));
    if SDIO_CONTROLLER
        .status
        .read()
        .contains(Status::ERR_CMD_TIMEOUT)
    {
        let mut test = 0x100000;
        let mut counter = 0;
        loop {
            counter += 1;
            let status = nwifi_send_cmd5(test);
            if status.successful() {
                let response = PORT.response[0];
                if !status.intersects(Status::ALL_ERRORS) {
                    if response & 0x80000000 > 0 {
                        if response & 0x100000 > 0 {
                            break;
                        }
                    }
                }
            }
            if counter > 0x10000 {
                return false;
            }
        }
        if !nwifi_send_cmd3(0).successful() {
            return false;
        }
        let rca = SDIO_CONTROLLER.response[0].read() & 0xFFFF0000;
        if !nwifi_send_cmd7(rca).successful() {
            return false;
        }
    } else {
        crate::nocash_write(">> NW SO");
    }
    true
}
pub unsafe fn init_nwifi_func0() -> bool {
    nwifi_write_func0(FuncReg::func0(0x012), 0x2);
    nwifi_write_func0(FuncReg::func0(0x007), 0x82);
    //PORT.option = 0;
    nwifi_write_func0(FuncReg::func0(0x008), 0x17);
    nwifi_write_func0(FuncReg::func0(0x110), 0x80);
    nwifi_write_func0(FuncReg::func0(0x111), 0x0);
    nwifi_write_func0(FuncReg::func0(0x010), 0x80);
    nwifi_write_func0(FuncReg::func0(0x011), 0x0);
    crate::swi_delay(0xF000);
    nwifi_write_func0(FuncReg::func0(0x002), 0x2);
    let mut counter = 0;
    while nwifi_read_func0(FuncReg::func0(3)) != Some(2) {
        counter += 1;
        if counter > 0x10000 {
            return false;
        }
    }

    true
}
const TEMP_BUF: *mut u8 = 0x2FF_B100 as *mut u8;
const TEMP_BUF_M14: *mut u8 = TEMP_BUF.wrapping_sub(14);
const TEMP_BUF_M16: *mut u8 = TEMP_BUF.wrapping_sub(16);
pub const STATUS: *mut u32 = TEMP_BUF.wrapping_sub(20) as *mut u32;

pub unsafe fn nwifi_write_func1w(addr: u16, data: u16) {}
pub struct FuncReg(u32);
impl FuncReg {
    pub const fn func0(reg: u16) -> Self {
        Self(((reg as u32) << 9))
    }

    pub const fn func1(reg: u16) -> Self {
        Self(((reg as u32) << 9) | (1 << 28))
    }
}
pub unsafe fn nwifi_send_cmd3(param: u32) -> Status {
    SDIO_CONTROLLER.send_command(&mut PORT, crate::mmc::Command::SetSendRelativeAddr, param)
    //nwifi_send(param, 0x403)
}
pub unsafe fn nwifi_send_cmd5(param: u32) -> Status {
    //nwifi_send(param, 0x705)
    SDIO_CONTROLLER.send_command(&mut PORT, crate::mmc::Command::SDIOOpCond, param)
}
pub unsafe fn nwifi_send_cmd7(param: u32) -> Status {
    //nwifi_send(param, 0x507)
    SDIO_CONTROLLER.send_command(&mut PORT, crate::mmc::Command::SelectCard, param)
}
static mut PORT: TMIOPort = TMIOPort::dsio();
pub unsafe fn nwifi_read_func0(reg: FuncReg) -> Option<u8> {
    SDIO_CONTROLLER
        .send_command(&mut PORT, crate::mmc::Command::SDIORegRW, reg.0)
        .successful()
        .then_some(PORT.response[0] as u8)
}
pub unsafe fn nwifi_write_func0(reg: FuncReg, byte: u8) -> bool {
    let send = (byte as u32) | reg.0 | (1 << 31);
    SDIO_CONTROLLER
        .send_command(&mut PORT, crate::mmc::Command::SDIORegRW, reg.0)
        .successful()
}
pub unsafe fn nwifi_init_complete() {
    return STATUS.write_volatile(1);
    (*(0x4004008 as *mut RW<u32>)).modify(|i| i | (1 << 19));
    (0x4004020 as *mut u16).write_volatile(1);
    init_nwifi_regs();

    if !init_nwifi_opcond() {
        STATUS.write_volatile(2);
    }

    if !init_nwifi_func0() {
        STATUS.write_volatile(3);
    }
}

pub unsafe fn nwifi_send(param: u32, cmd: u16) -> Option<(Status, u32)> {
    while SDIO_CONTROLLER.status.read().contains(Status::CMD_BUSY) {}
    SDIO_CONTROLLER.status.write(Status::empty());

    SDIO_CONTROLLER.param.write(param);
    //SDIO_CONTROLLER.stop_action.modify(|i| i & !1);
    SDIO_CONTROLLER.command.write(cmd);

    let mut status = SDIO_CONTROLLER.status.read();

    let mut counter = 0;
    while !status.intersects(Status::RESPONSE_END) {
        if status.intersects(Status::ALL_ERRORS) {
            return None;
        }
        if counter > 0x80000 {
            return None;
        }
        counter += 1;
    }
    if SDIO_CONTROLLER
        .status
        .read()
        .contains(Status::ERR_CMD_TIMEOUT)
    {
        None
    } else {
        Some((
            SDIO_CONTROLLER.status.read(),
            SDIO_CONTROLLER.response[0].read(),
        ))
    }
}

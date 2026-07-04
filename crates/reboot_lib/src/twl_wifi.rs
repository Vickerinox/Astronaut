use volatile_register::RW;

use crate::{
    ClockCnt, Control, DataControl32, SDIO_CONTROLLER, Status, StorageSector, TMIOPort, i2c::{I2C_HARDWARE, PowerRegister}, ndma::{GlobalControl, NDMAControl}, rtc::RTC_HARDWARE,
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
    I2C_HARDWARE.write_register(PowerRegister::WIFILED.into(), 0x13);
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

    (*(0x4004C04 as *mut RW<u16>)).modify(|i| i & !(1<<8)); //GPIO: TWL wifi mode on
    crate::swi_delay(5 * 134056);
    crate::i2c::I2C_HARDWARE.write_register(PowerRegister::WIFILED.into(), 0x13);
    (*(0x4004020 as *mut RW<u16>)).write(1); //SCFG: wifi on?
    
}
pub unsafe fn init_nwifi_opcond() -> u32 {
    //nwifi_read_func0(FuncReg::func0(4));
    //if SDIO_CONTROLLER
    //    .status
    //    .read()
    //    .contains(Status::ERR_CMD_TIMEOUT)
    // {
    let mut test = 0x0;
    let mut counter = 0;
    let status = SDIO_CONTROLLER.send_command(&mut PORT, crate::mmc::Command::GoIdleState, 0);
    if !status.successful() {
        return status.bits() | 1;
    }
    loop {
        loop {
            counter += 1;
            let status = SDIO_CONTROLLER.send_command(&mut PORT, crate::mmc::Command::SDIOOpCond, test);
            if status.successful() {
                break;
            }
            if counter > 0x100 {
                return status.bits() | 2;
            }
            
        }
        test = PORT.response[0] & 0xFFFFFF;

        if (PORT.response[0] & 0x80000000 > 0){
                break;
            }
    }
    /* 
    if !nwifi_send_cmd3(0).successful() {
        return false;
    }
    let rca = SDIO_CONTROLLER.response[0].read() & 0xFFFF0000;
    if !nwifi_send_cmd7(rca).successful() {
        return false;
    }
    */

    0
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
pub unsafe fn nwifi_write_func0(reg: FuncReg, byte: u8) -> Status {
    let send = (byte as u32) | reg.0 | (1 << 31);
    SDIO_CONTROLLER
        .send_command(&mut PORT, crate::mmc::Command::SDIORegRW, reg.0)
}
pub unsafe fn dsio_hw_init() {
    (*(0x4004008 as *mut RW<u32>)).modify(|i| i | (1<<19) | (1<<23));
     //wifi_card_wlan_init_bmi
    (*(0x4004C04 as *mut RW<u16>)).write(0);
    crate::swi_delay(5 * 134056);

    RTC_HARDWARE.transact(&[0x72u8, 0x80], &mut []);
    RTC_HARDWARE.transact(&[0x74u8, 0x00], &mut []);
    I2C_HARDWARE.write_register(PowerRegister::WIFILED.into(), 0x13);
    (*(0x4004020 as *mut RW<u16>)).write(1); //SCFG: wifi on?

    SDIO_CONTROLLER.data_control_32.modify(|i| (i & !DataControl32::ENABLE_RX_IRQ));
    SDIO_CONTROLLER.data_control_32.modify(|i| (i & !DataControl32::ENABLE_TX_IRQ));
    SDIO_CONTROLLER.data_control_32.modify(|i| (i | DataControl32::USE_DATA32 | DataControl32::CLEAR_FIFO_32));

    SDIO_CONTROLLER.data_control.modify(|i| (i & Control::MASK) | Control::USE_DATA32);
    SDIO_CONTROLLER.data_control.modify(|i| (i & !Control::from_bits_retain(0x20)));
    SDIO_CONTROLLER.block_len_32.write(128);
    
    SDIO_CONTROLLER.block_count_32.write(1);

    SDIO_CONTROLLER.soft_reset.modify(|i| i & !3);
    SDIO_CONTROLLER.soft_reset.modify(|i| i | 3);

    SDIO_CONTROLLER.irmask.write(Status::from_bits_retain(0xFFFFFFFF));

    SDIO_CONTROLLER.ext_card_detect_mask.modify(|i| i | 0xDB);
    SDIO_CONTROLLER.ext_card_detect_dat3_mask.modify(|i| i | 0xDB);

    SDIO_CONTROLLER.port_select.modify(|i| i & !3);

    SDIO_CONTROLLER.clock_control.write(ClockCnt::FREQ_262K);
    SDIO_CONTROLLER.options.write(0x40EE);

    SDIO_CONTROLLER.data_control_32.modify(|i| i & !DataControl32::from_bits_retain(0x8000));
    SDIO_CONTROLLER.data_control_32.modify(|i| i | DataControl32::RX_READY);
    SDIO_CONTROLLER.data_control_32.modify(|i| i & !DataControl32::RX_READY);
    
    SDIO_CONTROLLER.port_select.modify(|i| i & !3);

    SDIO_CONTROLLER.block_len.write(128);
    SDIO_CONTROLLER.stop_action.write(0x100);
}
unsafe fn mask16(reg: &mut RW<u16>, clear: u16, set: u16) {
    reg.modify(|i| (i & !clear) | set);
}
pub unsafe fn nwifi_init_bmi() -> Result<u32, u32> {
    CTX.clk_cnt = 0;
    CTX.bus_width = 1;
    CTX.block_size = 128;
    
   

    CTX.bus_width = 4;
    switch_device();

    while SDIO_CONTROLLER.status.read().contains(Status::CMD_BUSY) {}
    crate::swi_delay(0xF000);

    nwifi_read_func_byte(0, 0);

    if CTX.status & 4 > 0 {
        CTX.bus_width = 1;
        switch_device();
    }
    {
        wifi_base_init();
        if CTX.status & 4 > 0 {
            return Err(0xDEADBEEF);
        }
    }
    
    let cmd3 = 3 | 0x400;
    wifi_card_send_command(cmd3, 0);
    if CTX.status & 4 > 0 {
        return Err(1);
    }
    CTX.address = CTX.response[0] >> 16;

    let cmd7 = 7 | 0x500;
    wifi_card_send_command(cmd7, CTX.address << 16);
    if CTX.status & 4 > 0 {
        return Err(2);
    }

    if nwifi_write_func_byte(0, 2, 0) {
        return Err(3);
    }

    if nwifi_write_func_byte(0, 2, 2) {
        return Err(4);
    }
    crate::swi_delay(0xF000);

    let mut interface_cnt = nwifi_read_func_byte(0, 7);
    if CTX.status & 4 > 0 {
        return Err(5)
    }
    interface_cnt |= 0x82;

    if nwifi_write_func_byte(0, 7, interface_cnt) {
        return Err(6);
    }
    
    CTX.bus_width = 4;

    switch_device();


    if nwifi_write_func_byte(0, 0x12, 0x2) {
        return Err(7);
    }

    if nwifi_write_func_byte(0, 0x110, CTX.block_size as u8) {
        return Err(8);
    }
    if nwifi_write_func_byte(0, 0x111, (CTX.block_size >> 8) as u8) {
        return Err(9);
    }

    if nwifi_write_func_byte(0, 0x10, CTX.block_size as u8) {
        return Err(0x10);
    }
    if nwifi_write_func_byte(0, 0x11, (CTX.block_size >> 8) as u8) {
        return Err(0x11);
    }
    crate::swi_delay(0xF000);

    let revision = nwifi_read_func_byte(0, 0);
    if CTX.status & 4 > 0 {
        return Err(0x12);
    }
    nwifi_write_func_byte(0, 2, 2);
    loop {
        let read = nwifi_read_func_byte(0, 3);
        if read == 2 { break };
    }
    let manufacturer = nwifi_read_func_word(0, 0x1007);
    if CTX.status & 4 > 0 {
        return Err(0x13);
    }
    let Some(chip_id) = nwifi_read_intern_word(0x000040ec) else {return Err(0x14)};

    if CTX.status & 4 > 0 {
        return Err(0x15);
    }
    let interest_addr = if chip_id == 0x02000001 {
        //AR6002
        0x00500400
    } else {
        //AR601x
        0x00520000
    };

    let Some(bmi_ver) = nwifi_get_bmi_version() else {return Err(0x20)};

    if bmi_ver == 0xFFFFFFFF {
        return Err(0x21);
    }


    let is_fw_uploaded = nwifi_read_intern_word(interest_addr + 0x58);

    if CTX.status & 4 > 0 {
        return Err(0x16);
    }

    if nwifi_write_intern_word(0x4000, 0x100) {
        return Err(0x17);
    }

    crate::swi_delay(0x10000);

    let Some(reset_cause) = nwifi_read_intern_word(0x40C0) else { return Err(0x18)};

    if reset_cause != 2 {
        return Err(0x19);
    }
    Ok(interest_addr)

    /* 
    


    bmi_ver
    */
}
unsafe fn nwifi_get_bmi_version() ->  Option<u32> {
    if nwifi_write_mbox_u32(8, true) {
        return None;
    }

    let Some(version) = nwifi_read_mbox_word_timeout(10) else { return None };
    if version == 0xFFFFFFFF  {
        let len = nwifi_read_mbox_word();
        if len != 0xFFFFFFFF {
            let ver = nwifi_read_mbox_word();
            for _ in 0..((len/4)-2) {
                nwifi_read_mbox_word();
            }
            Some(ver)
        } else {
            Some(version)
        }
    } else {
        Some(version)
    }
}
unsafe fn nwifi_read_mbox_word_timeout(mut timeout: u32) -> Option<u32> {
    while (nwifi_read_func_byte(1, 0x405) & 1 == 0) && timeout != 0 { timeout -= 1};
    if (nwifi_read_func_byte(1, 0x405) & 1 == 0) {
        return None;
    }
    let mut val = 0;
    for i in 0..4 {
        val |= (nwifi_read_func_byte(1, 0xFF) as u32) << (i*8);
    }
    Some(val)
    
}
unsafe fn nwifi_read_mbox_word() -> u32 {
    while (nwifi_read_func_byte(1, 0x405) & 1 == 0) {}
    let mut val = 0;
    for i in 0..4 {
        val |= (nwifi_read_func_byte(1, 0xFF) as u32) << (i*8);
    }
    val    
}


unsafe fn nwifi_write_mbox_u32(mut val: u32, send_ird: bool) -> bool {
    let mut res = false;
    for i in 0..4 {
        let addr = if i == 3 && send_ird { 0xFF } else { 0 };
        res |= nwifi_write_func_byte(1, addr, val as u8);
        val >>= 8;
    }
    res
}
unsafe fn nwifi_read_intern_word(addr: u32) -> Option<u32> {
    (!nwifi_write_func_word(1, 0x47C, addr)).then_some(nwifi_read_func_word(1, 0x474))
}
unsafe fn nwifi_write_intern_word(addr: u32, value: u32) -> bool {
    nwifi_write_func_word(1, 0x474, value) |
    nwifi_write_func_word(1, 0x478, addr)
}
unsafe fn wifi_base_init() {
    SDIO_CONTROLLER.stop_action.write(0x100);
    let mut ocr = 0;

    let cmd5 = 5 | 0x700;

    loop {
        loop {
            wifi_card_send_command(cmd5, ocr);
            if CTX.status & 4 > 0 {
                return;
            }
            if CTX.status & 1 > 0 {
                break
            }
        }
        ocr = CTX.response[0] & 0xFFFFFF;

        if CTX.response[0] & 0x80000000 > 0 {
            break;
        }
    }
}
unsafe fn switch_device() {
    SDIO_CONTROLLER.port_select.modify(|i| (i & !3) | CTX.port);
    
    SDIO_CONTROLLER.clock_control.modify(|i| i & !ClockCnt::ENABLE);
    SDIO_CONTROLLER.clock_control.modify(|i| (i & !ClockCnt::from_bits_retain(0x2FF)) | ClockCnt::from_bits_retain(0x2FF & CTX.clk_cnt)  );
    SDIO_CONTROLLER.clock_control.modify(|i| i | ClockCnt::ENABLE);

    if CTX.bus_width == 4 {
        SDIO_CONTROLLER.options.modify(|i| i & !0x8000);
    } else {
        SDIO_CONTROLLER.options.modify(|i| i | 0x8000);
    }
}

static mut CTX: Context = Context {
    clk_cnt: 0,
    bus_width: 0,
    port: 0,
    address: 0,
    block_size: 0,
    status: 0,
    buffer: core::ptr::null_mut(),
    size: 0,
    stat: Status::empty(),
    errors: 0,
    response: [0; 4],
};
unsafe fn wifi_card_send_command(command: u16, arg: u32) {
    if CTX.block_size == 0 {
        CTX.block_size = 512;
        
    }
    let mut buf = CTX.buffer;
    let mut size = CTX.size;
    CTX.status = 0;
    CTX.errors = 0;
    let mut stat = Status::empty();
    let mut completion = Status::empty();
    let mut cnt32 = DataControl32::empty();

    if command & 0x700 != 0x300 {
        completion |= Status::RESPONSE_END;
    }
    if command & 0x800 > 0 {
        completion |= Status::DATA_END;
    }
    if CTX.buffer as u32 & 3 != 0 {
        CTX.status |= 4;
        return;
    }
    let timeout = 256*1024;
    let mut timer = 0;
    while SDIO_CONTROLLER.status.read().contains(Status::CMD_BUSY) {
        timer += 1;
        if timer > timeout {
            CTX.status |= 4;
            CTX.errors = SDIO_CONTROLLER.error_info.read();
            return;
        }
    }
    SDIO_CONTROLLER.status.write(Status::empty());
    SDIO_CONTROLLER.param.write(arg);

    SDIO_CONTROLLER.block_len.write(CTX.block_size);
    SDIO_CONTROLLER.block_count.write((CTX.size / CTX.block_size as u32) as u16);

    SDIO_CONTROLLER.block_len_32.write(CTX.block_size);
    SDIO_CONTROLLER.block_count_32.write((CTX.size / CTX.block_size as u32) as u16);
    
    
    let block_data = command & 0x2000 > 0;
    if (block_data) {
        SDIO_CONTROLLER.data_control.write(Control::USE_DATA32);
        if (command & 0x1000 == 0x1000) {
            SDIO_CONTROLLER.data_control_32.write(DataControl32::USE_DATA32 | DataControl32::ENABLE_RX_IRQ | DataControl32::CLEAR_FIFO_32);
        } else {
            SDIO_CONTROLLER.data_control_32.write(DataControl32::USE_DATA32 | DataControl32::ENABLE_TX_IRQ | DataControl32::CLEAR_FIFO_32);
        }
    }
    while SDIO_CONTROLLER.status.read().contains(Status::CMD_BUSY) {}
    SDIO_CONTROLLER.command.write(command);

    if (command & 0x1000 == 0x1000 && block_data && !buf.is_null()) {
        wifi_ndma_read(buf, size);
        return;
    } else if (command & 0x1000 == 0 && block_data && !buf.is_null()) {
        wifi_ndma_write(buf, size);
        return;
    }

    timer = 0;
    loop {
        timer += 1;
        if timer > timeout {
            CTX.status |= 4;
            CTX.errors = SDIO_CONTROLLER.error_info.read();
            return;
        }

        stat = SDIO_CONTROLLER.status.read();
        cnt32 = SDIO_CONTROLLER.data_control_32.read();

        if cnt32.contains(DataControl32::RX_READY) {

        }
        if stat.contains(Status::TX_REQUEST) {

        }
        if stat.intersects(Status::ALL_ERRORS) {
            CTX.status |= 4;
            break;
        }

        let end_cond = !stat.contains(Status::CMD_BUSY);
        if end_cond {
            if stat.contains(Status::RESPONSE_END) {
                CTX.status |= 1;
            }
            if stat.contains(Status::DATA_END) {
                CTX.status |= 2;
            }

            if stat & completion == completion {
                break;
            }
        }
    }

    CTX.stat = SDIO_CONTROLLER.status.read();
    CTX.errors = SDIO_CONTROLLER.error_info.read();

    SDIO_CONTROLLER.status.write(Status::empty());

    if command & 0x700 != 0x300 {
        CTX.response[0] = SDIO_CONTROLLER.response[0].read();
        CTX.response[1] = SDIO_CONTROLLER.response[1].read();
        CTX.response[2] = SDIO_CONTROLLER.response[2].read();
        CTX.response[3] = SDIO_CONTROLLER.response[3].read();
    }

}
unsafe fn wifi_ndma_read(buffer: *mut u8, size: u32) {

}
unsafe fn wifi_ndma_write(buffer: *mut u8, size: u32) {

}

pub struct Context {
    clk_cnt: u16,
    bus_width: u16,
    port: u16,
    address: u32,
    block_size: u16,
    status: u16,
    buffer: *mut u8,
    size: u32,
    stat: Status,
    errors: u32,
    response: [u32; 4],

}
pub unsafe fn nwifi_init_complete(wifi_version: u8, firmware: &mut [u8]) -> u32 {
    
    
    crate::ndma::NDMA_HARDWARE.channels[3].control.write(NDMAControl::empty());
    crate::ndma::NDMA_HARDWARE.global_control.write(GlobalControl::empty());
    dsio_hw_init();
    let interest = match nwifi_init_bmi() {
        Ok(area) => area,
        Err(err) => return err,
    };

    upload_wifi_firmware(wifi_version, firmware, interest)
}
fn find_firmware_for_card(version: u8, firmware: &[u8]) -> &[u8] {
    let Some(included_firmwares) = firmware.get(0xa2).copied() else { return &[] };
    let Some(firmware_index) = (0..included_firmwares as usize).into_iter().filter(|i| firmware.get(0xa4+8+(*i*32)).copied() == Some(version)).next() else { return &[]};
    let offset = 0xa4+(firmware_index*32);
    let Some(offset) = firmware.get(offset..).and_then(|i| i.first_chunk::<4>()) else {return &[]};
    let offset = u32::from_le_bytes(offset.clone()) as usize;
    let Some(firmware) = firmware.get(offset..) else { return &[]};
    firmware
}
unsafe fn upload_wifi_firmware(wifi_version: u8, firmware: &mut [u8], interest_area: u32) -> u32 {
    let firmware = find_firmware_for_card(wifi_version, firmware);
    if firmware.is_empty() {
        return 0x104;
    }

    let Ok((part_d, dest)) = get_wifi_part(firmware, FirmwarePart::PartD) else { return 0x105 };
    if wifi_card_upload_binary(dest, part_d) {
        return 0x106
    }


    let Ok((part_c, dest)) = get_wifi_part(firmware, FirmwarePart::PartC) else { return 0x107 };
    if wifi_card_upload_binary(dest, part_c) {
        return 0x108
    }

    if wifi_card_execute(dest + 0x400000, dest).is_none() {
        return 0x109
    }

    let Ok((part_a, dest)) = get_wifi_part(firmware, FirmwarePart::PartA) else { return 0x110 };
    if wifi_card_upload_binary_lz(dest, part_a) {
        return 0x111
    }

    let Ok((part_d, dest)) = get_wifi_part(firmware, FirmwarePart::PartD) else { return 0x112 };
    if wifi_card_upload_binary(dest, part_d) {
        return 0x113
    }


    let Ok((part_b, dest)) = get_wifi_part(firmware, FirmwarePart::PartB) else { return 0x114 };
    if wifi_card_upload_binary(dest, part_b) {
        return 0x115
    }

    if wifi_card_write_memory(interest_area+0x18, &dest.to_le_bytes()) {
        return 0x116
    }
    0
}
#[repr(u8)]
enum FirmwarePart {
    PartA = 0,
    PartB = 1,
    PartC = 2,
    PartD = 3,
}
fn get_wifi_part<'a>(firmware: &'a [u8], part: FirmwarePart) -> Result<(&'a [u8], u32), u32> {
    let parts = &firmware[4..][part as u8 as usize * 16..];
    let Some((offset, rem)) = parts.split_first_chunk::<4>() else { return Err(0x201)};
    let offset = u32::from_le_bytes(offset.clone()) as usize;
    let Some((len, rem)) = rem.split_first_chunk::<4>() else { return Err(0x202)};
    let len = u32::from_le_bytes(len.clone()) as usize;
    let Some((flags, rem)) = rem.split_first_chunk::<4>() else { return Err(0x203)};
    let Some((destination, rem)) = rem.split_first_chunk::<4>() else { return Err(0x204)};
    let destination = u32::from_le_bytes(destination.clone());
    Ok((&firmware[offset..offset+len], destination))
} 
unsafe fn wifi_card_upload_binary(addr: u32, binary: &[u8]) -> bool {
    const CHUNK_SIZE: usize = 0x1F0;
    for (i, chunk) in binary.chunks(CHUNK_SIZE).enumerate() {
        if wifi_card_write_memory(addr + (i*CHUNK_SIZE) as u32, chunk) {
            return true
        }
    }
    false
}
unsafe fn wifi_card_upload_binary_lz(addr: u32, binary: &[u8]) -> bool {
    if wifi_card_start_lz(addr) {
        return true
    }
    //while [0,0xFF].contains(&nwifi_read_func_byte(1, 0x450)) {}
    for chunk in binary.chunks(0x1F8) {
        if wifi_card_upload_lz(chunk) {
            return true;
        }
    }
    false
}

unsafe fn wifi_card_start_lz(addr: u32) -> bool {
    wifi_wait_count4();
    nwifi_write_mbox_u32(0xD, false) |
    nwifi_write_mbox_u32(addr, true)
}
unsafe fn wifi_card_upload_lz(data: &[u8]) -> bool {

    if data.is_empty() {return false}
    let len = (data.len() as u32 + 3) & !3;
    let mut iter = data.iter();

    wifi_wait_count4();
    if nwifi_write_mbox_u32(0xE, false) {
        return true
    }
    if nwifi_write_mbox_u32(len as u32, false) {
        return true
    }

    for _ in 0..(len-1) {
        let byte = iter.next().copied().unwrap_or(0);
        if nwifi_write_func_byte(1, 0, byte) {
            return true
        }
    }
    let byte = iter.next().copied().unwrap_or(0);
    if nwifi_write_func_byte(1, 0xFF, byte) {
        return true
    }

    false
}
unsafe fn wifi_wait_count4() {
    while [0xff, 0].contains(&nwifi_read_func_byte(1, 0x450)) {
        crate::swi::swi_delay(0x100);
    }
}
unsafe fn wifi_card_write_memory(addr: u32, data: &[u8]) -> bool {
    if data.is_empty() {return false}
    let len = (data.len() as u32 + 3) & !3;
    let mut iter = data.iter();
    wifi_wait_count4();
    if nwifi_write_mbox_u32(3, false) {
        return true
    }
    if nwifi_write_mbox_u32(addr, false) {
        return true
    }
    if nwifi_write_mbox_u32(len, false) {
        return true
    }

    for _ in 0..(len-1) {
        let byte = iter.next().copied().unwrap_or(0);
        if nwifi_write_func_byte(1, 0, byte) {
            return true
        }
    }
    let byte = iter.next().copied().unwrap_or(0);
    if nwifi_write_func_byte(1, 0xFF, byte) {
        return true
    }
    false
}
unsafe fn wifi_card_execute(addr: u32, arg: u32) -> Option<u32> {
    wifi_wait_count4();
    nwifi_write_mbox_u32(4, false);
    nwifi_write_mbox_u32(addr, false);
    nwifi_write_mbox_u32(arg, true);

    nwifi_read_mbox_word_timeout(2000)
}
unsafe fn nwifi_read_func_byte(func: u32, addr: u32) -> u8 {
    let arg = (func << 28) | ((addr & 0x1FFFF) << 9);
    wifi_card_send_command(52 | 0x400, arg);

    if CTX.status & 4 > 0 {
        return 0xFF;
    }
    CTX.response[0] as u8
}
unsafe fn nwifi_read_func_halfword(func: u32, addr: u32) -> u16 {
    nwifi_read_func_byte(func, addr) as u16 | ((nwifi_read_func_byte(func, addr+1) as u16) << 8)
}
unsafe fn nwifi_read_func_word(func: u32, addr: u32) -> u32 {
    nwifi_read_func_halfword(func, addr) as u32 | ((nwifi_read_func_halfword(func, addr+2) as u32) << 16)
}
unsafe fn nwifi_write_func_byte(func: u32, addr: u32, value: u8) -> bool {
    let arg = (func << 28) | ((addr & 0x1FFFF) << 9) | (1<<31) | (value as u32);
    wifi_card_send_command(52 | 0x400, arg);

    if CTX.status & 4 > 0 {
        return true;
    }
    false
}
unsafe fn nwifi_write_func_halfword(func: u32, addr: u32, value: u16) -> bool {
    nwifi_write_func_byte(func, addr+1, (value >> 8) as u8)
    | nwifi_write_func_byte(func, addr,value as u8)
    
}

unsafe fn nwifi_write_func_word(func: u32, addr: u32, value: u32) -> bool {
    nwifi_write_func_halfword(func, addr+2, (value >> 16) as u16)
    | nwifi_write_func_halfword(func, addr,value as u16)
    
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

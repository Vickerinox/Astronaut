use reboot_lib::{
    mmc::Command, swi_delay, swi_halt, ClockCnt, Control, DataControl32, Status, MMC,
    MMC_CONTROLLER,
};

pub static mut MMC_DEVICE: Device = Device {
    port: TMIOPort {
        num: 1,
        sd_clk_ctrl: ClockCnt::empty(),
        sd_blocklen: 0,
        sd_option: 0,
        buffer: core::ptr::null_mut(),
        blocks: 0,
        response: [0; 4],
    },
    kind: DeviceType::MMC,
    protection: 0,
    rca: 0,
    ccc: 0,
    sectors: 0,
    status: 0,
    cid: [0; 4],
};

pub struct TMIOPort {
    num: u8,
    sd_clk_ctrl: ClockCnt,
    sd_blocklen: u16,
    sd_option: u16,
    buffer: *mut u32,
    blocks: u16,
    pub response: [u32; 4],
}
pub struct Device {
    pub port: TMIOPort,
    kind: DeviceType,
    protection: u8,
    rca: u16,
    ccc: u16,
    sectors: u32,
    status: u32,
    cid: [u32; 4],
}

static mut MMCSD_STATUS: Status = Status::empty();

unsafe fn tmio_mmc_irq() {
    //update our status copy
    MMCSD_STATUS |= MMC_CONTROLLER.status.read();
    //acknowledge all irq's except CMD_BUSY (it disables itself)
    MMC_CONTROLLER.status.write(Status::CMD_BUSY);
}
pub unsafe fn init_all() -> Result<(), Status> {
    tmio_mmc_init();
    sdmmc_init(&mut MMC_DEVICE)?;
    Ok(())
}
pub fn read_mmc_sectors(data: *mut [reboot_lib::StorageSector], sector: u32) -> Result<(), Status> {
    unsafe {
        let blocks = data.len();
        let sector = match MMC_DEVICE.kind {
            DeviceType::SDSC => sector << 9,
            DeviceType::SDHC => sector,
            DeviceType::MMC => sector << 9,
            DeviceType::MMCHC => sector,
        };
        MMC_DEVICE.port.buffer = data as *mut _;
        MMC_DEVICE.port.blocks = blocks as _;

        match MMC_DEVICE
            .port
            .send_command(Command::ReadMutliBlocks, sector)
        {
            Status::EMPTY => Ok(()),
            err => Err(err),
        };
        Ok(())
    }
}
unsafe fn tmio_mmc_init() {
    reboot_lib::set_interrupt_function(reboot_lib::ARM7Interrupt::SDMMC, tmio_mmc_irq);
    reboot_lib::enable_interrupt(reboot_lib::ARM7Interrupt::SDMMC);

    let MMC {
        port_select,
        block_count,
        irmask,
        clock_control,
        block_len,
        options,
        sdio_mode,
        sdio_mask,
        data_control,
        soft_reset,
        ext_sdio_irq,
        ext_card_detect_mask,
        ext_card_detect_dat3_mask,
        data_control_32,
        block_len_32,
        block_count_32,
        ..
    } = &*MMC_CONTROLLER;
    soft_reset.write(0);
    soft_reset.write(1);

    data_control_32.write(DataControl32::CLEAR_FIFO_32 | DataControl32::USE_DATA32);
    block_len_32.write(512);
    block_count_32.write(1);
    data_control.write(Control::USE_DATA32);

    port_select.write(0);
    block_count.write(1);
    irmask.write(
        Status::UNKNOWN
            | Status::TX_REQUEST
            | Status::RX_READY
            | Status::DAT3_INSERT
            | Status::DAT3_REMOVE,
    );
    clock_control.write(ClockCnt::FREQ_262K);
    block_len.write(512);
    options.write((1 << 15) | (1 << 14) | (11 << 4) | 8);
    ext_card_detect_mask.write(0xFFFF);
    ext_card_detect_dat3_mask.write(0xFFFF);

    sdio_mode.write(0);
    sdio_mask.write(0xFFFF);
    ext_sdio_irq.write((1 << 8) | (1 << 9) | (1 << 10));
}

unsafe fn sdmmc_init(device: &mut Device) -> Result<(), Status> {
    device.port.init(1);
    device.port.powerup();

    match device.port.send_command(Command::SendStatus, 0) {
        Status::EMPTY => return Ok(()),
        err => return Err(err),
    };

    match device.go_idle_state() {
        Status::EMPTY => (),
        err => return Err(err),
    };

    match device.go_idle_state() {
        Status::EMPTY => (),
        err => return Err(err),
    };

    device.port.sd_clk_ctrl = ClockCnt::FREQ_262K | ClockCnt::ENABLE | ClockCnt::AUTO_STOP;
    loop {
        match device
            .port
            .send_command(Command::MMCSendOptionalCondition, 0x100000)
        {
            Status::EMPTY => {
                if device.port.response[0] & 0x80000000 > 0 {
                    break;
                }
            }
            _ => (),
        }
    }

    let device_type = if device.port.response[0] & (1 << 30) > 0 {
        DeviceType::MMCHC
    } else {
        DeviceType::MMC
    };
    match device.port.send_command(Command::AllSendCID, 0) {
        Status::EMPTY => (),
        err => return Err(err),
    }
    match device
        .port
        .send_command(Command::SetSendRelativeAddr, 0x10000)
    {
        Status::EMPTY => (),
        err => return Err(err),
    }

    device.port.sd_clk_ctrl = ClockCnt::FREQ_16M | ClockCnt::ENABLE | ClockCnt::AUTO_STOP;
    match device.port.send_command(Command::SendCSD, 0x10000) {
        Status::EMPTY => (),
        err => return Err(err),
    }
    let spec = device.parse_csd(device_type);
    match device.port.send_command(Command::SendCID, 0x10000) {
        Status::EMPTY => (),
        err => return Err(err),
    }
    match device.port.send_command(Command::SelectCard, 0x10000) {
        Status::EMPTY => (),
        err => return Err(err),
    }

    match device.port.send_command(Command::SetBlockLen, 0x200) {
        Status::EMPTY => (),
        err => return Err(err),
    }
    return Ok(());
    match device.port.send_command(Command::SwitchMMC, 0x03B70100) {
        Status::EMPTY => (),
        err => return Err(err),
    }
    device.port.sd_option = 0;

    /*
    let device_type = match device.init_idle_state() {
        Ok(a) => a,
        Err(err) => return Err(err),
    };
    */
    device.kind = device_type;
    /*
     device.init_ready_state()?;
     let rca = match device.init_ident_state(device_type) {
         Ok(rca) => {
             device.rca = rca;
             rca
         }
         Err(err) => return Err(err),
     };

     let spec = match device.init_standby_state(device_type, rca) {
         Ok(spec) => spec,
         Err(err) => return Err(err),
     };
     device.init_trans_state(device_type, rca, spec);
    */

    Ok(())
}

impl TMIOPort {
    pub fn init(&mut self, port_num: u8) {
        self.num = port_num;
        self.sd_clk_ctrl = ClockCnt::FREQ_262K;
        self.sd_blocklen = 512;
        self.sd_option = (1 << 15) | (1 << 14) | (11 << 4) | 8;
    }
    pub unsafe fn powerup(&mut self) {
        self.sd_clk_ctrl = ClockCnt::ENABLE | ClockCnt::FREQ_262K;
        set_port(self);
        swi_delay(ClockCnt::FREQ_262K.bits() as u32 * 74 / 4);
    }
    pub unsafe fn send_command(&mut self, command: Command, argument: u32) -> Status {
        MMCSD_STATUS = Status::EMPTY;

        set_port(self);
        let blocks = self.blocks;
        MMC_CONTROLLER.block_count.write(blocks);
        MMC_CONTROLLER.stop_action.write(1 << 8);
        MMC_CONTROLLER.param.write(argument);

        let buffer = self.buffer;
        let control = match (buffer.is_null(), command.reads_data()) {
            (true, _) => DataControl32::CLEAR_FIFO_32 | DataControl32::USE_DATA32,
            (false, true) => {
                DataControl32::CLEAR_FIFO_32
                    | DataControl32::USE_DATA32
                    | DataControl32::ENABLE_RX_IRQ
            }
            (false, false) => {
                DataControl32::CLEAR_FIFO_32
                    | DataControl32::USE_DATA32
                    | DataControl32::ENABLE_TX_IRQ
            }
        };

        MMC_CONTROLLER.data_control_32.write(control);
        MMC_CONTROLLER.status.write(Status::empty());
        MMC_CONTROLLER.command.write(command as u16);
        while !core::ptr::read_volatile(&MMCSD_STATUS).contains(Status::RESPONSE_END) {
            swi_halt();
        }
        get_response(self);
        if command.transmits_data() {
            if buffer.is_null() {
                while !core::ptr::read_volatile(&MMCSD_STATUS).contains(Status::DATA_END) {
                    swi_halt();
                }
            } else {
                return do_cpu_transfer(self, command);
            }
        }
        MMCSD_STATUS & Status::ALL_ERRORS
    }

    unsafe fn send_app_command(&mut self, cmd: Command, arg: u32, rca: u32) -> Status {
        match self.send_command(Command::AppCommand, rca) {
            Status::EMPTY => self.send_command(cmd, arg),
            a => a,
        }
    }
}
impl Device {
    unsafe fn go_idle_state(&mut self) -> Status {
        self.port.send_command(Command::GoIdleState, 0)
    }
    unsafe fn init_idle_state(&mut self) -> Result<DeviceType, Status> {
        let res = self.port.send_command(Command::SendIfCondition, 0x1AA);

        //unexpected SD response
        if res.is_empty() {
            if self.port.response[0] != 0x1AA {
                return Err(Status::from_bits_retain(0xDEAD0FFF));
            }
        }
        //unexpected MMC response
        else if !res.contains(Status::ERR_CMD_TIMEOUT) {
            return Err(res);
        }

        let op_cond_arg = (1 << 20) | ((res.bits() << 8) ^ (1 << 30));

        let res = self
            .port
            .send_app_command(Command::AppSendOpCondition, op_cond_arg, 0);
        let dev_type = if res.contains(Status::ERR_CMD_TIMEOUT) {
            DeviceType::MMC //MMC
        } else if res.is_empty() {
            DeviceType::SDSC //SDSC
        } else {
            return Err(Status::UNKNOWN | Status::from_bits_retain(1 << 28));
        };
        let dev_type = if dev_type == DeviceType::MMC {
            let mut tries = 200;
            let mut ocr;
            loop {
                let res = self
                    .port
                    .send_command(Command::MMCSendOptionalCondition, (1 << 20) | (2 << 29));
                if !res.is_empty() {
                    return Err(res);
                };

                ocr = self.port.response[0];
                if tries < 1 {
                    return Err(Status::ERR_CMD_TIMEOUT);
                }
                if ocr & (1 << 31) > 0 {
                    break;
                }
                swi_delay(41000);
                tries -= 1;
            }
            if (ocr & (1 << 20)) == 0 {
                return Err(Status::UNKNOWN | Status::from_bits_retain(1 << 26));
            }
            if ocr & (2 << 29) > 0 {
                DeviceType::MMCHC
            } else {
                DeviceType::MMC
            }
        } else {
            let mut tries = 200;
            let mut ocr;
            loop {
                ocr = self.port.response[0];
                if ocr & (1 << 31) > 0 {
                    break;
                }
                if tries < 1 {
                    return Err(Status::ERR_CMD_TIMEOUT);
                }

                let res = self
                    .port
                    .send_app_command(Command::AppSendOpCondition, op_cond_arg, 0);
                if !res.is_empty() {
                    return Err(res);
                };

                swi_delay(41000);
                tries -= 1;
            }
            if (ocr & (1 << 20)) == 0 {
                return Err(Status::UNKNOWN | Status::from_bits_retain(1 << 26));
            }
            if ocr & (2 << 29) > 0 {
                DeviceType::SDHC
            } else {
                DeviceType::SDSC
            }
        };
        Ok(dev_type)
    }

    unsafe fn init_ready_state(&mut self) -> Result<(), Status> {
        let res = self.port.send_command(Command::AllSendCID, 0);
        if !res.is_empty() {
            return Err(res);
        };
        self.cid[0] = self.port.response[0];
        self.cid[1] = self.port.response[1];
        self.cid[2] = self.port.response[2];
        self.cid[3] = self.port.response[3];
        Ok(())
    }

    unsafe fn init_ident_state(&mut self, dev_type: DeviceType) -> Result<u16, Status> {
        if dev_type == DeviceType::MMC || dev_type == DeviceType::MMCHC {
            let res = self
                .port
                .send_command(Command::SetSendRelativeAddr, 1 << 16);
            if res.is_empty() {
                return Ok(1);
            } else {
                return Err(res);
            }
        } else {
            let res = self.port.send_command(Command::SetSendRelativeAddr, 0);
            if res.is_empty() {
                return Ok((self.port.response[0] >> 16) as u16);
            } else {
                return Err(res);
            }
        }
    }
    unsafe fn init_standby_state(&mut self, dev_type: DeviceType, rca: u16) -> Result<u8, Status> {
        let arg = (rca as u32) << 16;
        let res = self.port.send_command(Command::SendCSD, arg);
        if !res.is_empty() {
            return Err(res);
        }
        let spec = self.parse_csd(dev_type);
        let res = self.port.send_command(Command::SelectCard, arg);
        if !res.is_empty() {
            return Err(res);
        }

        Ok(spec)
    }
    unsafe fn init_trans_state(
        &mut self,
        dev_type: DeviceType,
        rca: u16,
        spec: u8,
    ) -> Result<(), Status> {
        let rca = (rca as u32) << 16;
        if dev_type == DeviceType::MMC || dev_type == DeviceType::MMCHC {
            if spec > 3 {
                let bus_width_arg = (3 << 24) | (183 << 16) | (1 << 8) | (0);
                let res = self
                    .port
                    .send_command(Command::SwitchFunction, bus_width_arg);
                if !res.is_empty() {
                    return Err(res);
                }
                self.port.sd_option = 0 | (1 << 14) | (11 << 4) | 8;
                if dev_type == DeviceType::MMCHC {
                    //TODO!!!!
                }
            }
        } else {
            let res = self
                .port
                .send_app_command(Command::AppSetClearCardSelect, 0, rca);
            if !res.is_empty() {
                return Err(res);
            }

            let res = self.port.send_app_command(Command::AppSetBusWidth, 2, rca);
            if !res.is_empty() {
                return Err(res);
            }
            self.port.sd_option = 0 | (1 << 14) | (11 << 4) | 8;
        }
        Ok(())
    }
    fn parse_csd(&mut self, dev_type: DeviceType) -> u8 {
        let csd = &self.port.response;
        let structure = extract_bits(csd, 126, 2) as u8;
        let retu = extract_bits(csd, 122, 4) as u8;
        self.ccc = extract_bits(csd, 84, 12) as u16;
        let sectors = if structure == 0 || dev_type == DeviceType::MMC {
            let read_bl_len = extract_bits(csd, 80, 4); // [83:80]
            let c_size = extract_bits(csd, 62, 12); // [73:62]
            let c_size_mult = extract_bits(csd, 47, 3); // [49:47]
            (c_size + 1) << (c_size_mult + 2 + read_bl_len - 9)
        } else if dev_type != DeviceType::MMCHC {
            let c_size = extract_bits(csd, 48, 28);
            (c_size + 1) << 10
        } else {
            0
        };
        self.sectors = sectors;
        let prot = (extract_bits(csd, 12, 1) << 1) as u8 | (extract_bits(csd, 13, 1) << 2) as u8;
        self.protection = prot;
        return retu;
    }
}
const fn extract_bits(response: &[u32; 4], start: u32, size: u32) -> u32 {
    let mask = if size < 32 { 1 << size } else { 0u32 }.wrapping_sub(1);
    let off = 3 - (start / 32);
    let shift = start & 31;
    let mut res = response[off as usize] >> shift;
    if size + shift > 32 {
        res |= response[(off - 1) as usize] << ((32 - shift) & 31);
    }
    res & mask
}
#[inline]
unsafe fn do_cpu_transfer(port: &mut TMIOPort, command: Command) -> Status {
    let block_len = MMC_CONTROLLER.block_len.read();
    let mut block_count = port.blocks;
    let mut status = core::ptr::read_volatile(&MMCSD_STATUS);
    if command.reads_data() {
        while block_count > 0 {
            status = core::ptr::read_volatile(&MMCSD_STATUS);
            if MMC_CONTROLLER
                .data_control_32
                .read()
                .contains(DataControl32::RX_READY)
                || status.intersects(Status::RX_READY)
            {
                let ptr = port.buffer;
                for i in 0..(port.sd_blocklen >> 2) {
                    (ptr as *mut u32)
                        .add(i as usize)
                        .write_volatile(MMC_CONTROLLER.data_fifo_32.read());
                }
            } else if status.intersects(Status::ALL_ERRORS) {
                return status;
            } else if !status.contains(Status::CMD_BUSY) {
                return status;
            } else {
                swi_halt();
            }
        }
    } else {
        while !MMCSD_STATUS.intersects(Status::ALL_ERRORS) && block_count > 0 {
            if MMC_CONTROLLER
                .data_control_32
                .read()
                .contains(DataControl32::TX_READY)
            {
                let block_end = port.buffer.byte_add(block_len as usize);
                while port.buffer < block_end {
                    MMC_CONTROLLER
                        .data_fifo_32
                        .write(port.buffer.read_volatile());
                    port.buffer = port.buffer.add(1);
                }
                block_count -= 1;
            } else {
                swi_halt();
            }
        }
    }
    status
}
unsafe fn get_response(port: &mut TMIOPort) {
    port.response[0] = MMC_CONTROLLER.response[0].read();
    port.response[1] = MMC_CONTROLLER.response[1].read();
    port.response[2] = MMC_CONTROLLER.response[2].read();
    port.response[3] = MMC_CONTROLLER.response[3].read();
}
unsafe fn set_port(port: &mut TMIOPort) {
    MMC_CONTROLLER.port_select.write(port.num as u16);
    MMC_CONTROLLER.clock_control.write(port.sd_clk_ctrl);
    MMC_CONTROLLER.block_len.write(port.sd_blocklen);
    MMC_CONTROLLER.options.write(port.sd_option);
    MMC_CONTROLLER.block_len_32.write(port.sd_blocklen);
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum DeviceType {
    SDSC,
    SDHC,
    MMC,
    MMCHC,
}

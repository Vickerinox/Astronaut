use core::num::NonZeroU32;

use crate::Control;

use super::{ClockCnt, Command, Status, TMIOPort, MMC_CONTROLLER};

#[repr(u8)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum DeviceType {
    EMMC = 0,
    HCEMMC = 1,
    SDSC = 2,
    SDHC = 3,
}

bitflags::bitflags! {
    #[derive(Default)]
    pub struct Protection: u8 {
        const SLIDER   = (1);       // SD card write protection slider.
        const TEMP     = (1<<1);    // Temporary write protection (CSD).
        const PERM     = (1<<2);    // Permanent write protection (CSD).
        const PASSWORD = (1<<3);    // (e)MMC/SD card is password protected.
    }
}
pub struct Info {
    kind: Option<DeviceType>,
    protection: Protection,
    buswidth: u8,
    rca: u16,
    command_class_support: u16,
    sectors: u32,
    clock: u32,
    cid: [u32; 4],
}

#[derive(Default)]
pub struct Device {
    port: TMIOPort,
    kind: Option<DeviceType>,
    protection: Protection,
    rca: u16,
    command_class_support: u16,
    sectors: Option<NonZeroU32>,
    status: u32,
    cid: [u32; 4],
}

#[repr(u8)]
#[derive(PartialEq, Clone, Copy)]
pub enum DeviceSelect {
    SDCardSlot = 0,
    EMMC = 1,
}

impl Device {
    const fn sd_card() -> Self {
        Self {
            port: TMIOPort::init(0),
            kind: None,
            protection: Protection::empty(),
            rca: 0,
            command_class_support: 0,
            sectors: None,
            status: 0,
            cid: [0; 4],
        }
    }
    const fn nand() -> Self {
        Self {
            port: TMIOPort::init(1),
            kind: Some(DeviceType::EMMC),
            protection: Protection::empty(),
            rca: 0,
            command_class_support: 0,
            sectors: None,
            status: 0,
            cid: [0; 4],
        }
    }
}

#[repr(u8)]
pub enum DeviceInitializationError {
    AlreadyInitialized,
    IdleStateTransitionError,
    BadIfConditionResponse,
    IdentificationFail,
}
fn sdmmc_ignore() {}
unsafe fn init_sdmmc_general() {
    crate::set_interrupt_function(crate::ARM7Interrupt::SDMMC, sdmmc_ignore as _);
    crate::enable_interrupt(crate::ARM7Interrupt::SDMMC);
}
static mut DEVICES: [Device; 2] = [Device::sd_card(), Device::nand()];
pub unsafe fn init_sdmmc(device_number: DeviceSelect) -> Result<(), Status> {
    if device_number == DeviceSelect::SDCardSlot {}
    let dev = &mut DEVICES[device_number as u8 as usize];

    MMC_CONTROLLER.tmio_powerup(&mut dev.port);
    /*
    dev.port.clock = ClockCnt::FREQ_16M;
    MMC_CONTROLLER.tmio_set_port(&mut dev.port);
    let res = MMC_CONTROLLER.send_command(&mut dev.port, Command::StopTransmission, 0);
    let res = MMC_CONTROLLER.send_command(&mut dev.port, Command::SendStatus, 0);
    let status = MMC_CONTROLLER.response[0].read();

    if status & 0xF00 == 0x900 {
        return Ok(())
    }

    return Err(Status::from_bits_retain(0x80000000 | res.bits()));
    */
    MMC_CONTROLLER.data_control.write(Control::USE_DATA32);
    MMC_CONTROLLER
        .data_control_32
        .write(super::DataControl32::CLEAR_FIFO_32 | super::DataControl32::USE_DATA32);

    match device_number {
        DeviceSelect::SDCardSlot => {
            dev.port.clock = super::ClockCnt::ENABLE | super::ClockCnt::FREQ_262K;
            crate::swi_delay(0x2000);
            let res = MMC_CONTROLLER.send_command(&mut dev.port, Command::GoIdleState, 0);
            if !res.successful() {
                return Err(res);
            }
            let res = MMC_CONTROLLER.send_command(&mut dev.port, Command::SendIfCondition, 0x1AA);
            match res {
                Status::EMPTY => {
                    if dev.port.response[0] != 0x1AA {
                        return Err(Status::from_bits_retain(dev.port.response[0] | 0x80000000));
                    }
                }
                Status::ERR_CMD_TIMEOUT => (),
                err => return Err(err),
            }
            let op_cond_arg = (1 << 20) | ((res.bits() << 8) ^ (1 << 30));
            let mut kind = DeviceType::SDSC;
            let res = send_app_command(&mut dev.port, Command::AppSendOpCondition, op_cond_arg, 0);
            match res {
                Status::EMPTY => (),
                Status::ERR_CMD_TIMEOUT => return Err(Status::from_bits_retain(54321)),
                err => return Err(err),
            }
            let mut ocr;
            let mut tries = 0;
            loop {
                if tries == 200 {
                    return Err(Status::from_bits_retain(6666));
                }
                ocr = dev.port.response[0];
                if (ocr & (1 << 31) > 0) {
                    break;
                }
                let res =
                    send_app_command(&mut dev.port, Command::AppSendOpCondition, op_cond_arg, 0);
                if !res.successful() {
                    return Err(Status::from_bits_retain(32123));
                }
                crate::swi::swi_delay(0x20BA * 5);
                tries += 1;
            }
            if (ocr & (1 << 20)) == 0 {
                return Err(Status::from_bits_retain(123123));
            };
            if (ocr & (1 << 30)) > 0 {
                kind = DeviceType::SDHC
            };

            match MMC_CONTROLLER.send_command(&mut dev.port, Command::AllSendCID, 0) {
                Status::EMPTY => (),
                err => return Err(err),
            }

            match MMC_CONTROLLER.send_command(&mut dev.port, Command::SDSendRelativeAddr, 0) {
                Status::EMPTY => (),
                err => return Err(err),
            }
            let rca = dev.port.response[0];
            dev.port.clock = ClockCnt::ENABLE | ClockCnt::FREQ_16M | ClockCnt::AUTO_STOP;

            match MMC_CONTROLLER.send_command(&mut dev.port, Command::SendCSD, rca) {
                Status::EMPTY => (),
                err => return Err(err),
            }

            match MMC_CONTROLLER.send_command(&mut dev.port, Command::SelectCard, rca) {
                Status::EMPTY => (),
                err => return Err(err),
            }
            match send_app_command(&mut dev.port, Command::AppSetClearCardSelect, 0, rca) {
                Status::EMPTY => (),
                err => return Err(err),
            }
            /*
            match send_app_command(&mut dev.port, Command::AppSetBusWidth, 2, rca) {
                Status::EMPTY => (),
                err => return Err(err),
            }
            */
            match MMC_CONTROLLER.send_command(&mut dev.port, Command::SendStatus, rca) {
                Status::EMPTY => (),
                err => return Err(err),
            }
            dev.kind = Some(kind);
            return Ok(());
        }
        DeviceSelect::EMMC => {
            dev.port.clock = super::ClockCnt::ENABLE | super::ClockCnt::FREQ_262K;

            crate::swi_delay(0xf000);
            let res = MMC_CONTROLLER.send_command(&mut dev.port, Command::GoIdleState, 0);
            if !res.successful() {
                return Err(Status::from_bits_retain(1) | res);
            }
            crate::swi_delay(0xf000);
            let mut card_calming_down = true;
            while card_calming_down {
                if MMC_CONTROLLER
                    .send_command(&mut dev.port, Command::MMCSendOptionalCondition, (1 << 20))
                    .successful()
                {
                    card_calming_down = MMC_CONTROLLER.response[0].read() & 0x80000000 == 0
                }
            }

            let res = MMC_CONTROLLER.send_command(&mut dev.port, Command::AllSendCID, 0);
            if !res.successful() {
                return Err(Status::from_bits_retain(2) | res);
            }
            let res =
                MMC_CONTROLLER.send_command(&mut dev.port, Command::SetSendRelativeAddr, 0x10000);
            if !res.successful() {
                return Err(Status::from_bits_retain(0xBA) | res);
            }
            let res = MMC_CONTROLLER.send_command(&mut dev.port, Command::SelectCard, 0x10000);
            if !res.successful() {
                return Err(Status::from_bits_retain(4) | res);
            }

            dev.port.clock =
                super::ClockCnt::ENABLE | super::ClockCnt::FREQ_16M | super::ClockCnt::AUTO_STOP;

            if !res.successful() {
                return Err(Status::from_bits_retain(5) | res);
            }

            return Ok(());
        }
    }

    //nocash_write("powerup!");
    MMC_CONTROLLER.tmio_powerup(&mut dev.port);
    match go_idle_state(&mut dev.port) {
        Ok(_) => (),
        Err(a) => return Err(a),
    }
    let device_kind = init_idle_state(&mut dev.port)?;
    dev.port.clock =
        super::ClockCnt::ENABLE | super::ClockCnt::AUTO_STOP | super::ClockCnt::FREQ_262K;
    go_ready_state(dev)?;
    let rca = go_ident_state(dev, device_kind)?;
    dev.port.clock =
        super::ClockCnt::ENABLE | super::ClockCnt::AUTO_STOP | super::ClockCnt::FREQ_16M;
    let spec_version = go_standby_state(dev, device_kind, rca)?;
    dev.kind = Some(device_kind);
    Ok(())
}
unsafe fn go_standby_state(device: &mut Device, kind: DeviceType, rca: u32) -> Result<u8, Status> {
    /*
    let res = MMC_CONTROLLER.send_command(&mut device.port, CommandNumber::SendCSD, rca);
    if !res.is_empty() {
        return Err(res)
    }
    let csd = parse_csd(device, kind);
    */
    let res = MMC_CONTROLLER.send_command(&mut device.port, Command::SelectCard, rca);
    if !res.is_empty() {
        return Err(res);
    }
    let locked = device.port.response[0] & (1 << 25) >> 22;
    device
        .protection
        .insert(Protection::from_bits_retain(locked as u8));

    Ok(0)
}
unsafe fn parse_csd(device: &mut Device, kind: DeviceType) -> u8 {
    let Device {
        port,
        command_class_support,
        sectors,
        protection,
        ..
    } = device;
    let resp = &port.response;
    let structure = extract_bits(resp, 126, 2) as u8;
    let spec = extract_bits(resp, 122, 4) as u8;
    *command_class_support = extract_bits(resp, 84, 12) as u16;
    let sector_count = if structure == 0 || kind == DeviceType::EMMC {
        let bl_len = extract_bits(resp, 80, 4);
        let c_size = extract_bits(resp, 62, 12);
        let c_size_mult = extract_bits(resp, 47, 3);
        let count = (c_size + 1) << (c_size_mult + 2 + bl_len - 9);
        Some(NonZeroU32::new_unchecked(count))
    } else if kind != DeviceType::HCEMMC {
        let c_size = extract_bits(resp, 48, 28);
        let count = (c_size + 1) << 10;
        Some(NonZeroU32::new_unchecked(count))
    } else {
        None
    };
    let bits = (resp[0] >> 11) & 0b110;
    *protection = Protection::from_bits_retain(bits as u8);
    *sectors = sector_count;
    spec
}
fn extract_bits(resp: &[u32; 4], start: usize, size: usize) -> u32 {
    let mask: u32 = if size < 32 { 1 << size as u32 } else { 0u32 }.wrapping_sub(1);
    let off = 3 - (start >> 5);
    let shift = start & 0x31;
    let mut res = resp[off] >> shift;
    if size + shift > 32 {
        res |= resp[off - 1] << ((32 - shift) & 0x31);
    }
    res & mask
}
unsafe fn go_ident_state(device: &mut Device, kind: DeviceType) -> Result<u32, Status> {
    let (res, rca) = match kind {
        DeviceType::EMMC | DeviceType::HCEMMC => {
            let rca = 1 << 16;
            let res =
                MMC_CONTROLLER.send_command(&mut device.port, Command::SetSendRelativeAddr, rca);
            (res, rca)
        }
        DeviceType::SDSC | DeviceType::SDHC => {
            let res =
                MMC_CONTROLLER.send_command(&mut device.port, Command::SetSendRelativeAddr, 0);
            let rca = device.port.response[0] & 0xFFFF0000;
            (res, rca)
        }
    };
    match res.is_empty() {
        true => Ok(rca),
        false => Err(res),
    }
}
unsafe fn go_ready_state(device: &mut Device) -> Result<(), Status> {
    let res = MMC_CONTROLLER.send_command(&mut device.port, Command::AllSendCID, 0);
    if res.is_empty() {
        device.cid = device.port.response;
        Ok(())
    } else {
        Err(res)
    }
}
unsafe fn go_idle_state(port: &mut TMIOPort) -> Result<(), Status> {
    match MMC_CONTROLLER.send_command(port, Command::GoIdleState, 0) {
        Status::EMPTY => Ok(()),
        a => Err(a),
    }
}
unsafe fn init_idle_state(port: &mut TMIOPort) -> Result<DeviceType, Status> {
    let res = MMC_CONTROLLER.send_command(port, Command::SendIfCondition, (1 << 8) | 0xAA);
    if res != Status::empty() {
        return Err(res);
    }
    let app_command_arg = (1 << 20) | (res.bits() << 8 ^ (1 << 30));
    let res = send_app_command(port, Command::AppSendOpCondition, app_command_arg, 0);
    let is_mmc = match port.port_num {
        1 => true,
        0 => false,
        _ => return Err(res),
    };

    if is_mmc {
        let mut ocr = 0;
        for _ in 0..200 {
            let res = MMC_CONTROLLER.send_command(
                port,
                Command::MMCSendOptionalCondition,
                (1 << 20) | (2 << 29),
            );
            if !res.is_empty() {
                return Err(res);
            }
            ocr = port.response[0];
            //confirmed working and supports specified voltage
            if ocr & (1 << 31) > 0 {
                if ocr & (1 << 20) == 0 {
                    return Err(res);
                } else if ocr & (1 << 30) > 0 {
                    return Ok(DeviceType::HCEMMC);
                } else {
                    return Ok(DeviceType::EMMC);
                }
            }
            //5 MS
            crate::swi::swi_delay(41890);
        }
    } else {
        let mut ocr;
        for _ in 0..200 {
            ocr = port.response[0];
            //confirmed working and supports specified voltage
            if (ocr & ((1 << 31) | (1 << 20))) > 0 {
                if ocr & (1 << 30) > 0 {
                    return Ok(DeviceType::SDHC);
                } else {
                    return Ok(DeviceType::SDSC);
                }
            }
            //5 MS
            crate::swi::swi_delay(41890);
            let res = send_app_command(port, Command::AppSendOpCondition, app_command_arg, 0);

            if !res.is_empty() {
                return Err(res);
            }
        }
    }
    Err(res)
}
unsafe fn send_app_command(port: &mut TMIOPort, cmd: Command, arg: u32, rca: u32) -> Status {
    match MMC_CONTROLLER.send_command(port, Command::AppCommand, rca) {
        Status::EMPTY => MMC_CONTROLLER.send_command(port, cmd, arg),
        a => a,
    }
}
pub unsafe fn device_response(device: DeviceSelect) -> [u32; 4] {
    let device = &mut DEVICES[device as u8 as usize];
    device.port.response.clone()
}
pub unsafe fn read_sectors(
    device: DeviceSelect,
    sector: u32,
    buf: *mut [crate::StorageSector],
) -> Result<(), Status> {
    let device = &mut DEVICES[device as u8 as usize];
    device.port.buffer = buf as *mut _;
    device.port.buffer_len = buf.len();

    let sector = match device.kind {
        None => return Err(Status::all()),
        Some(DeviceType::SDSC) | Some(DeviceType::EMMC) => sector << 9,
        _ => sector,
    };

    let res = MMC_CONTROLLER.send_command(&mut device.port, Command::ReadMutliBlocks, sector);
    if res.successful() {
        Ok(())
    } else {
        Err(res)
    }
}
pub unsafe fn write_sd_sectors(
    sector: u32,
    buf: *mut [crate::StorageSector],
) -> Result<(), Status> {
    let device = &mut DEVICES[DeviceSelect::SDCardSlot as u8 as usize];
    device.port.buffer = buf as *mut _;
    device.port.buffer_len = buf.len();

    let sector = match device.kind {
        None => return Err(Status::all()),
        Some(DeviceType::SDSC) | Some(DeviceType::EMMC) => sector << 9,
        _ => sector,
    };

    let res = MMC_CONTROLLER.send_command(&mut device.port, Command::WriteMultiBlocks, sector);
    if res.successful() {
        Ok(())
    } else {
        Err(res)
    }
}

pub unsafe fn nocash_write(str: &str) {
    #[cfg(target_arch = "arm")]
    core::arch::asm!(
        /*
        "ldr r3, =3f",

        "4:",
        "sub r2, 1",
        "ldrb r4, [r1, r2]",
        "strb r4, [r3, r2]",
        "cmp r2, 0",
        "bne 4b",

        */
        "mov r12, r12",
        "b 2f",
        ".hword  0x6464",          // second ID
        ".hword  0 ",              // flags
        "3:",
        ".word 0x44444444",
        ".word 0",
        ".word 0",
        ".word 0",
        ".word 0",
        ".word 0",
        ".word 0",
        ".word 0",
        ".word 0",
        ".word 0",
        ".word 0",
        ".word 0",
        ".word 0",
        ".word 0",
        ".word 0",
        ".word 0",
        ".word 0",
        ".word 0",
        ".word 0",
        ".word 0",
        ".word 0",
        ".word 0",
        ".word 0",
        ".word 0",
        ".word 0",
        ".word 0",
        ".word 0",
        ".word 0",
        ".word 0",
        ".word 0",
        "2:",
        in("r1") str as *const str as *const (),
        in("r2") str.len() as u32,
        out("r3") _,
        out("r4") _,
    );
    /*
    const NOCASH_OUT_CHR: *mut u32 = 0x4fffa1c as *mut u32;
    const NOCASH_OUT_STR: *mut u8 = 0x4fffa10 as *mut u8;

    for byte in str.as_bytes() {
        NOCASH_OUT_CHR.write_volatile(*byte as u32);
    }
    */
}

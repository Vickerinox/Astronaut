use core::num::NonZeroU32;

use super::{Command, Status, TMIOPort, MMC_CONTROLLER};

#[repr(u8)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum DeviceType {
    EMMC,
    HCEMMC,
    SDSC,
    SDHC,
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
    const fn sd_card(dev_num: u8) -> Self {
        Self {
            port: TMIOPort::init(dev_num),
            kind: None,
            protection: Protection::empty(),
            rca: 0,
            command_class_support: 0,
            sectors: None,
            status: 0,
            cid: [0; 4],
        }
    }
    const fn nand(dev_num: u8) -> Self {
        Self {
            port: TMIOPort::init(dev_num),
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

static mut DEVICES: [Device; 2] = [Device::sd_card(0), Device::nand(1)];
pub unsafe fn init_sdmmc(device_number: DeviceSelect) -> Result<(), Status> {
    
    MMC_CONTROLLER.soft_reset.write(0);
    let dev = &mut DEVICES[device_number as u8 as usize];
    MMC_CONTROLLER.soft_reset.write(1);
    MMC_CONTROLLER.data_control_32.write(super::DataControl32::CLEAR_FIFO_32 | super::DataControl32::USE_DATA32);



    

    MMC_CONTROLLER.tmio_powerup(&mut dev.port);

    match device_number {
        DeviceSelect::SDCardSlot => {
            
        },
        DeviceSelect::EMMC => {
            dev.port.clock = super::ClockCnt::ENABLE | super::ClockCnt::FREQ_262K;
            let res = MMC_CONTROLLER.send_command(&mut dev.port, Command::GoIdleState, 0);
            if !res.successful() {
                return Err(Status::from_bits_retain(1) | res);
            }
            crate::swi_delay(0xf000);
            let mut card_calming_down = true;
            while card_calming_down {
                if MMC_CONTROLLER.send_command(&mut dev.port, Command::MMCSendOptionalCondition, (1<<20)).successful() {
                    card_calming_down = MMC_CONTROLLER.response[0].read() & 0x80000000 == 0
                }
            }
            
            let res = MMC_CONTROLLER.send_command(&mut dev.port, Command::AllSendCID, 0);
            if !res.successful() {
                return Err(Status::from_bits_retain(2) | res);
            }
            let res = MMC_CONTROLLER.send_command(&mut dev.port, Command::SetSendRelativeAddr, 0x20000);
            if !res.successful() {
                return Err(Status::from_bits_retain(0xBA) | res);
            }
            let res = MMC_CONTROLLER.send_command(&mut dev.port, Command::SelectCard, 0x20000);
            if !res.successful() {
                return Err(Status::from_bits_retain(4) | res);
            }
            dev.port.clock = super::ClockCnt::ENABLE | super::ClockCnt::FREQ_8M | super::ClockCnt::AUTO_STOP;
            let res = MMC_CONTROLLER.send_command(&mut dev.port, Command::SetBlockLen, 0x200);
            if !res.successful() {
                return Err(Status::from_bits_retain(5) | res);
            }
            return Ok(())
        },
    }

     
    //nocash_write("powerup!");
    MMC_CONTROLLER.tmio_powerup(&mut dev.port);
    match go_idle_state(&mut dev.port) {
        Ok(_) => (),
        Err(a) => return Err(a),
    }
    let device_kind = init_idle_state(&mut dev.port)?;
    dev.port.clock = super::ClockCnt::ENABLE | super::ClockCnt::AUTO_STOP | super::ClockCnt::FREQ_262K;
    go_ready_state(dev)?;
    let rca = go_ident_state(dev, device_kind)?;
    dev.port.clock = super::ClockCnt::ENABLE | super::ClockCnt::AUTO_STOP | super::ClockCnt::FREQ_16M;
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
            let res = MMC_CONTROLLER.send_command(
                &mut device.port,
                Command::SetSendRelativeAddr,
                rca,
            );
            (res, rca)
        }
        DeviceType::SDSC | DeviceType::SDHC => {
            let res = MMC_CONTROLLER.send_command(
                &mut device.port,
                Command::SetSendRelativeAddr,
                0,
            );
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

pub unsafe fn read_sectors(
    device: DeviceSelect,
    sector: u32,
    buf: *mut [crate::StorageSector],
) -> Result<(), Status> {
    let device = &mut DEVICES[device as u8 as usize];
    device.port.buffer = buf as *mut _;

    let sector = match device.kind {
        None => return Err(Status::all()),
        Some(DeviceType::SDSC) | Some(DeviceType::EMMC) => {
            sector << 9
        },
        _ => sector,
    };
    
    let res = MMC_CONTROLLER.send_command(&mut device.port, Command::ReadMutliBlocks, sector);
    if res.successful() {
        Ok(())
    } else {
        Err(res)
    }
}

pub unsafe fn nocash_write(str: &str) {
    const NOCASH_OUT_CHR: *mut u8 = 0x4fffa1c as *mut u8;
    for byte in str.as_bytes() {
        NOCASH_OUT_CHR.write_volatile(*byte);
    }
}

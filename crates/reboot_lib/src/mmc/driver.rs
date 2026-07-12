use core::num::NonZeroU32;

use alloc::rc;

use crate::{Control, MMC, StorageSector};

use super::{ClockCnt, Command, Status, TMIOPort, MMC_CONTROLLER};

#[repr(u8)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum DeviceType {
    EMMC = 0,
    HCEMMC = 1,
    SDSC = 2,
    SDHC = 3,
}
impl DeviceType {
    pub fn is_mmc(&self) -> bool {
        match self {
            DeviceType::EMMC | DeviceType::HCEMMC => true,
            DeviceType::SDSC | DeviceType::SDHC => false,
        }
    }
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
    rca: u32,
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
            port: TMIOPort::sdmmc::<0>(),
            kind: None,
            protection: Protection::empty(),
            rca: 0,
            sectors: None,
            status: 0,
            cid: [0; 4],
        }
    }
    const fn nand() -> Self {
        Self {
            port: TMIOPort::sdmmc::<1>(),
            kind: None,
            protection: Protection::empty(),
            rca: 0,
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
#[cfg(feature = "arm7i")]
unsafe fn init_sdmmc_general() {
    crate::set_interrupt_function(crate::Interrupt::SDMMC, sdmmc_ignore as _);
    crate::enable_interrupt(crate::Interrupt::SDMMC);
}
static mut DEVICES: [Device; 2] = [Device::sd_card(), Device::nand()];
pub unsafe fn check_sdmmc(device_number: DeviceSelect) -> Status {
    let dev = &mut DEVICES[device_number as u8 as usize];
    MMC_CONTROLLER.send_command(&mut dev.port, Command::SendStatus, 0)
}
#[repr(u16)]
pub enum InitSDMMCError {
    FailedIdleCommand = 1,
    FailedIdleState = 2,
    FailedCMD41 = 3,
    FailedPlaceholder = 0xffff,
    FailedReadyState = 4,
    FailedIdentState = 5,
    FailedStandbyState = 6,

    FailedIdleVoltageRange = 7,
    FailedIdleVoltageError = 8,

    FailedIdleOpCondSD = 9,
    FailedIdleOpCondMMC = 10,

    FailedIdleOpCondSD2 = 11,
    FailedIdleOpCondVoltage = 12,
    FailedTranState = 13,
    FailedOcrTimeout = 14,

    CID = 15,
    RelAddr = 16,

    Select = 17,
    CSD = 18,
    Desel = 19,

    Status = 20,
    BusWidthSD = 21,
    StatusVerify = 22,
}
#[cfg(feature = "arm7i")]
pub unsafe fn init_sdmmc(device_number: DeviceSelect) -> Result<(), InitSDMMCError> {
    if device_number == DeviceSelect::SDCardSlot {}
    let dev = &mut DEVICES[device_number as u8 as usize];

    dev.port.clock = super::ClockCnt::ENABLE | super::ClockCnt::FREQ_262K;
    crate::swi_delay(0x2000);
    let res = MMC_CONTROLLER.send_command(&mut dev.port, Command::GoIdleState, 0);
    if !res.successful() {
        return Err(InitSDMMCError::FailedIdleCommand);
    }
    let res = MMC_CONTROLLER.send_command(&mut dev.port, Command::SendIfCondition, 0x100 | 0xAA);
    let maybe_mmc = match res {
        Status::EMPTY => {
            if dev.port.response[0] != (0x100 | 0xAA) {
                // WARN: dangerous???? But gotta do it?????
                true //return Err(InitSDMMCError::FailedIdleVoltageRange);
            } else {
                false
            }
        }
        Status::ERR_CMD_TIMEOUT => true,
        err => return Err(InitSDMMCError::FailedIdleVoltageError),
    };
    let op_cond_arg = (1 << 20) | ((res.bits() << 8) ^ (1 << 30));
    let mut kind = DeviceType::SDSC;
    let res = send_app_command(&mut dev.port, Command::AppSendOpCondition, op_cond_arg, 0);
    let is_mmc = match res {
        Ok(Status::EMPTY) => false, //No$GBA compatability, maybe kind of dangerous? really the voltage should be verified here but idk.
        Ok(Status::ERR_CMD_TIMEOUT) => true,
        Err(Status::ERR_CMD_TIMEOUT) => true,
        err => return Err(InitSDMMCError::FailedReadyState),
    };
    let mut ocr;
    let mut tries = 0;
    if maybe_mmc || is_mmc {
        loop {
            if tries == 200 {
                return Err(InitSDMMCError::FailedOcrTimeout);
            }
            let res = MMC_CONTROLLER.send_command(
                &mut dev.port,
                Command::MMCSendOptionalCondition,
                (2 << 29) | (1 << 20),
            );
            if res != Status::empty() {
                return Err(InitSDMMCError::FailedIdleOpCondMMC);
            }
            ocr = dev.port.response[0];
            if (ocr & (1 << 31) > 0) {
                break;
            }
            crate::swi::swi_delay(0x20BA * 5);
            tries += 1;
        }
        if (ocr & (1 << 20)) == 0 {
            return Err(InitSDMMCError::FailedIdleOpCondVoltage);
        };
        if (ocr & (2 << 29)) > 0 {
            kind = DeviceType::HCEMMC
        } else {
            kind = DeviceType::EMMC
        };
    } else {
        loop {
            if tries == 200 {
                return Err(InitSDMMCError::FailedOcrTimeout);
            }
            ocr = dev.port.response[0];
            if (ocr & (1 << 31)) > 0 {
                break;
            }
            let res = send_app_command(&mut dev.port, Command::AppSendOpCondition, op_cond_arg, 0);
            if res != Ok(Status::empty()) {
                return Err(InitSDMMCError::FailedIdleOpCondSD);
            }
            crate::swi::swi_delay(0x20BA * 5);
            tries += 1;
        }
        if (ocr & (1 << 20)) == 0 {
            return Err(InitSDMMCError::FailedIdleOpCondSD2);
        };
        if (ocr & (1 << 30)) > 0 {
            kind = DeviceType::SDHC
        } else {
            kind = DeviceType::SDSC
        }
    }

    dev.port.clock = ClockCnt::FREQ_262K | ClockCnt::ENABLE | ClockCnt::AUTO_STOP;

    match MMC_CONTROLLER.send_command(&mut dev.port, Command::AllSendCID, 0) {
        Status::EMPTY => (),
        err => return Err(InitSDMMCError::CID),
    }
    let rca = if kind.is_mmc() {
        match MMC_CONTROLLER.send_command(&mut dev.port, Command::Test, 0x10000) {
            Status::EMPTY => (),
            err => return Err(InitSDMMCError::RelAddr),
        }
        0x10000
    } else {
        match MMC_CONTROLLER.send_command(&mut dev.port, Command::Test, 0) {
            Status::EMPTY => (),
            err => return Err(InitSDMMCError::RelAddr),
        }

        dev.port.response[0] & 0xFFFF0000
    };

    match MMC_CONTROLLER.send_command(&mut dev.port, Command::SendCSD, rca) {
        Status::EMPTY => (),
        err => return Err(InitSDMMCError::CSD),
    }

    match MMC_CONTROLLER.send_command(&mut dev.port, Command::SelectCard, rca) {
        Status::EMPTY => (),
        err => return Err(InitSDMMCError::Select),
    }

    dev.rca = rca;
    dev.port.clock = ClockCnt::ENABLE | ClockCnt::FREQ_16M | ClockCnt::AUTO_STOP;

    crate::swi_delay(0x208 * 5);

    if kind.is_mmc() {
    } else {
        match send_app_command(&mut dev.port, Command::AppSetClearCardSelect, 0, rca) {
            Ok(Status::EMPTY) => (),
            err => return Err(InitSDMMCError::Desel),
        }
        match send_app_command(&mut dev.port, Command::AppSetBusWidth, 2, rca) {
            Ok(Status::EMPTY) => (),
            err => return Err(InitSDMMCError::BusWidthSD),
        }
        dev.port.set_bus_width(4);
    }
    match MMC_CONTROLLER.send_command(&mut dev.port, Command::SendStatus, rca) {
        Status::EMPTY => {

            //if dev.port.response[0]&0x1e00 != 0x800 {
            //    return Err(InitSDMMCError::Status)
            //}
        }
        err => return Err(InitSDMMCError::StatusVerify),
    }
    dev.kind = Some(kind);
    Ok(())
}

#[cfg(feature = "arm7i")]
unsafe fn send_app_command(
    port: &mut TMIOPort,
    cmd: Command,
    arg: u32,
    rca: u32,
) -> Result<Status, Status> {
    match MMC_CONTROLLER.send_command(port, Command::AppCommand, rca) {
        Status::EMPTY => Ok(MMC_CONTROLLER.send_command(port, cmd, arg)),
        a => Err(a),
    }
}

#[cfg(feature = "arm7i")]
pub unsafe fn device_response(device: DeviceSelect) -> [u32; 4] {
    let device = &mut DEVICES[device as u8 as usize];
    device.port.response.clone()
}

#[cfg(feature = "arm7i")]
pub unsafe fn read_sectors(
    device: DeviceSelect,
    sector: u32,
    buf: *mut [crate::StorageSector],
) -> Result<(), Status> {
    use crate::StorageSector;

    let device = &mut DEVICES[device as u8 as usize];
    device.port.buffer = core::slice::from_raw_parts_mut(buf as *mut StorageSector as *mut u32, buf.len() * 128);

    let sector = match device.kind {
        None => return Err(Status::all()),
        Some(DeviceType::SDSC) | Some(DeviceType::EMMC) => sector << 9,
        _ => sector,
    };
    let res = MMC_CONTROLLER.send_command(&mut device.port, Command::ReadMutliBlocks, sector);

    if (res).successful() {
        Ok(())
    } else {
        let res2 = MMC_CONTROLLER.send_command(&mut device.port, Command::StopTransmission, 0);
        Err(res | res2)
    }
}
pub unsafe fn write_sd_sectors(
    sector: u32,
    buf: *mut [crate::StorageSector],
) -> Result<(), Status> {
    let device = &mut DEVICES[DeviceSelect::SDCardSlot as u8 as usize];
    device.port.buffer = core::slice::from_raw_parts_mut(buf as *mut StorageSector as *mut u32, buf.len() * 128);

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

use fatfs_embedded::fatfs::{
    diskio::{DiskResult, FatFsDriver, IoctlCommand},
    RawFileSystem,
};
use reboot_lib::{
    arm9_check_sdmmc, arm9_init_sdmmc, StorageSector, ENGINE_A_PALETTES, ENGINE_B_PALETTES,
};
use super::{mbr,nand::BasicSDMMCCursor};
use crate::{ read_encrypted_nand, read_sd_card, transmute_slice};

pub struct SDMMCDriver {
    pub nand_controller: Option<BasicSDMMCCursor<'static>>,
    pub sdmc_controller: Option<BasicSDMMCCursor<'static>>,
    pub nand_error: u32,
    pub sdmc_error: u32,
}
impl SDMMCDriver {
    pub const fn new() -> Self {
        Self {
            nand_controller: None,
            sdmc_controller: None,
            nand_error: 0,
            sdmc_error: 0,
        }
    }

    unsafe fn try_mount_sd(&mut self) -> Option<BasicSDMMCCursor<'static>> {
        let sd_buffer = core::slice::from_raw_parts_mut(0x2FE8000 as *mut StorageSector, 4);
        read_sd_card(sd_buffer, 0).ok()?;
        let mbr: &mbr::MBR = &*(transmute_slice(sd_buffer));
        if !mbr.has_valid_signature() {
            return None;
        }
        let twl_lba = core::ptr::read_unaligned(core::ptr::addr_of!(mbr.partitions[0].lba));
        let sd_buffer =
            core::slice::from_raw_parts_mut(0x2FE8000 as *mut reboot_lib::StorageSector, 32);
        match BasicSDMMCCursor::new(sd_buffer, twl_lba, false) {
            Ok(succ) => Some(succ),
            Err(code) => {
                self.sdmc_error = code;
                None
            }
        }
    }

    unsafe fn try_mount_nand(&mut self) -> Option<BasicSDMMCCursor<'static>> {
        let nand_buffer = core::slice::from_raw_parts_mut(0x2FEC000 as *mut StorageSector, 4);
        read_encrypted_nand(nand_buffer, 0).ok()?;
        let mbr: &mbr::MBR = &*(transmute_slice(nand_buffer));
        if !mbr.has_valid_signature() {
            return None;
        }
        let twl_lba = core::ptr::read_unaligned(core::ptr::addr_of!(mbr.partitions[0].lba));
        read_encrypted_nand(nand_buffer, twl_lba).ok()?;
        let _twl_size =
            core::ptr::read_unaligned(core::ptr::addr_of!(mbr.partitions[0].sector_count));
        let nand_buffer =
            core::slice::from_raw_parts_mut(0x2FEC000 as *mut reboot_lib::StorageSector, 32);
        match BasicSDMMCCursor::new(nand_buffer, twl_lba, true) {
            Ok(succ) => Some(succ),
            Err(code) => {
                self.nand_error = code;
                None
            }
        }
    }
}

impl FatFsDriver for SDMMCDriver {
    fn disk_status(&mut self, drive: u8) -> u8 {
        match unsafe { arm9_check_sdmmc(drive) } {
            Ok(()) => 0,
            Err(_any) => 1,
        }
    }
    fn disk_initialize(&mut self, drive: u8) -> u8 {
        match unsafe { arm9_init_sdmmc(drive) } {
            Ok(()) => match drive {
                1 => {
                    self.sdmc_controller = unsafe { self.try_mount_sd() };
                    self.sdmc_controller.is_none() as u8
                }
                2 => {
                    self.nand_controller = unsafe { self.try_mount_nand() };
                    self.nand_controller.is_none() as u8
                }
                _ => 1,
            },
            Err(bits) => {
                match drive {
                    1 => self.sdmc_error = bits.get(),
                    2 => self.nand_error = bits.get(),
                    _ => (),
                }
                1
            }
        }
    }
    fn disk_ioctl(&mut self, data: &mut IoctlCommand) -> DiskResult {
        match data {
            IoctlCommand::CtrlSync(_) => {
                if let Some(flusha) = &mut self.sdmc_controller {
                    flusha.flush();
                }
                DiskResult::Ok
            }
            IoctlCommand::GetSectorCount(_) => DiskResult::ParameterError,
            IoctlCommand::GetSectorSize(_) => DiskResult::ParameterError,
            IoctlCommand::GetBlockSize(_) => DiskResult::ParameterError,
        }
    }
    fn disk_read(&mut self, drive: u8, mut buffer: &mut [u8], sector: u32) -> DiskResult {
        let Some(controller) = (match drive {
            1 => &mut self.sdmc_controller,
            2 => &mut self.nand_controller,
            _ => return DiskResult::ParameterError,
        }) else {
            return DiskResult::NotReady;
        };
        let new_pos = sector;
        if controller.seek(new_pos) != Ok(new_pos) {
            return DiskResult::NotReady;
        }
        while !buffer.is_empty() {
            let Ok(progress) = controller.read(buffer) else {
                return DiskResult::NotReady;
            };
            let Some(remaining_buffer) = buffer.get_mut(progress..) else {
                return DiskResult::Error;
            };
            buffer = remaining_buffer;
        }
        DiskResult::Ok
    }
    fn disk_write(&mut self, drive: u8, mut buffer: &[u8], sector: u32) -> DiskResult {
        let Some(controller) = (match drive {
            1 => &mut self.sdmc_controller,
            2 => return DiskResult::WriteProtected, //&mut self.nand_controller,
            _ => return DiskResult::ParameterError,
        }) else {
            return DiskResult::NotReady;
        };
        let new_pos = sector;
        if controller.seek(new_pos) != Ok(new_pos) {
            return DiskResult::NotReady;
        }
        while !buffer.is_empty() {
            let Ok(progress) = controller.write(buffer) else {
                return DiskResult::NotReady;
            };
            let Some(remaining_buffer) = buffer.get(progress..) else {
                return DiskResult::Error;
            };
            buffer = remaining_buffer;
        }
        DiskResult::Ok
    }
}

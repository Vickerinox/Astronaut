use super::*;

pub type DSTATUS = BYTE;
pub const STA_NOINIT: DSTATUS =	0x01;	/* Drive not initialized */
pub const STA_NODISK: DSTATUS =	0x02;	/* No medium in the drive */
pub const STA_PROTECT: DSTATUS = 0x04;	/* Write protected */

pub const SECTOR_SIZE: usize = 512;

pub type DRESULT = cty::c_uint;
pub const DRESULT_RES_OK: DRESULT = 0;
pub const DRESULT_RES_ERROR: DRESULT = 1;
pub const DRESULT_RES_WRPRT: DRESULT = 2;
pub const DRESULT_RES_NOTRDY: DRESULT = 3;
pub const DRESULT_RES_PARERR: DRESULT = 4;

/* Generic command (Used by FatFs) */
const CTRL_SYNC: BYTE = 0;	/* Complete pending write process (needed at FF_FS_READONLY == 0) */
const GET_SECTOR_COUNT: BYTE = 1;	/* Get media size (needed at FF_USE_MKFS == 1) */
const GET_SECTOR_SIZE: BYTE = 2;	/* Get sector size (needed at FF_MAX_SS != FF_MIN_SS) */
const GET_BLOCK_SIZE: BYTE = 3;	/* Get erase block size (needed at FF_USE_MKFS == 1) */
const CTRL_TRIM: BYTE = 4;	/* Inform device that the data on the block of sectors is no longer used (needed at FF_USE_TRIM == 1) */

#[no_mangle]
pub unsafe extern "C" fn disk_status(pdrv: BYTE) -> DSTATUS {
    if let Some(driver) = &DRIVER {
        driver.disk_status(pdrv)
    } else {
        STA_NOINIT
    }
}

#[no_mangle]
pub unsafe extern "C" fn disk_initialize(pdrv: BYTE) -> DSTATUS {
    if let Some(driver) = &mut DRIVER {
        driver.disk_initialize(pdrv)
    } else {
        STA_NOINIT
    }
}

#[no_mangle]
pub unsafe extern "C" fn disk_read(pdrv: BYTE, buff: *mut BYTE, sector: LBA_t, count: UINT) -> DRESULT {
    if let Some(driver) = &mut DRIVER {
        let buffer = &mut *ptr::slice_from_raw_parts_mut(buff, (count as usize) * SECTOR_SIZE);
        driver.disk_read(pdrv, buffer, sector) as DRESULT
    } else {
        DRESULT_RES_ERROR
    }
}

#[no_mangle]
pub unsafe extern "C" fn disk_write(pdrv: BYTE, buff: *const BYTE, sector: LBA_t, count: UINT) -> DRESULT {
    if let Some(driver) = &mut DRIVER {
        let buffer = &*ptr::slice_from_raw_parts(buff, (count as usize) * SECTOR_SIZE);
        driver.disk_write(pdrv, buffer, sector) as DRESULT
    } else {
        DRESULT_RES_ERROR
    }
}

#[no_mangle]
pub unsafe extern "C" fn disk_ioctl(_lun: BYTE, cmd: BYTE, buff: *mut cty::c_void) -> DRESULT {
    if let Some(driver) = &DRIVER {
        let mut data = match cmd {
            CTRL_SYNC => IoctlCommand::CtrlSync(()),
            GET_SECTOR_COUNT => IoctlCommand::GetSectorCount(0),
            GET_SECTOR_SIZE => IoctlCommand::GetSectorSize(0),
            GET_BLOCK_SIZE => IoctlCommand::GetBlockSize(0),
            CTRL_TRIM => panic!("CTRL_TRIM is not implemented."),
            _ => panic!("An invalid FatFS IOCTL command was received.")
        };
        driver.disk_ioctl(&mut data);
        match data {
            IoctlCommand::GetBlockSize(value) => buff.copy_from(ptr::addr_of!(value).cast(), 4),
            IoctlCommand::GetSectorSize(value) => buff.copy_from(ptr::addr_of!(value).cast(), 2),
            IoctlCommand::GetSectorCount(value) => buff.copy_from(ptr::addr_of!(value).cast(), 4),
            _ => ()
        }
        DRESULT_RES_OK
    } else {
        DRESULT_RES_ERROR
    }
}

#[no_mangle]
pub unsafe extern "C" fn get_fattime() -> DWORD {
    
    #[cfg(feature = "chrono")]
    if let Some(driver) = &DRIVER {
        let timestamp = driver.get_fattime();
        let year = timestamp.year() as u32;
        let month = timestamp.month();
        let day = timestamp.day();
        let hour = timestamp.hour();
        let minute = timestamp.minute();
        let second = timestamp.second();
        let result = (year - 80) << 25 | month << 21 | day << 16 | hour << 11 | minute << 5 | second << 1;
        return result
    } else {
        return 0
    }

    #[cfg(not(feature = "chrono"))]
    return 0
}
mod diskio_bindings;

use crate::fatfs::diskio::diskio_bindings::*;
use crate::fatfs::*;
use core::ptr;

pub enum IoctlCommand {
    CtrlSync(()),
    GetSectorCount(DWORD),
    GetSectorSize(WORD),
    GetBlockSize(DWORD),
}

pub enum DiskResult {
    Ok = DRESULT_RES_OK as isize,
    Error = DRESULT_RES_ERROR as isize,
    WriteProtected = DRESULT_RES_WRPRT as isize,
    NotReady = DRESULT_RES_NOTRDY as isize,
    ParameterError = DRESULT_RES_PARERR as isize,
}

pub enum DiskStatus {
    Ok = 0,
    NotInitialized = STA_NOINIT as isize,
    NoDisk = STA_NODISK as isize,
    WriteProtected = STA_PROTECT as isize,
}

/// Implement this trait for a block storage device, such as an SDMMC driver.
/// When feature `chrono` is enabled time must also be supplied.
pub trait FatFsDriver {
    fn disk_status(&mut self, drive: u8) -> u8;
    fn disk_initialize(&mut self, drive: u8) -> u8;
    fn disk_read(&mut self, drive: u8, buffer: &mut [u8], sector: u32) -> DiskResult;
    fn disk_write(&mut self, drive: u8, buffer: &[u8], sector: u32) -> DiskResult;
    fn disk_ioctl(&mut self, data: &mut IoctlCommand) -> DiskResult;
    /*
    #[cfg(feature = "chrono")]
    fn get_fattime(&mut self) -> NaiveDateTime;
    */
}

/// Installed driver singleton. A call to `install()` places the driver here.
/// Only one driver instance is supported.
static mut DRIVER: Option<&'static mut dyn FatFsDriver> = None;
 
/// Installs a driver for the file system. Only one driver can be installed at a time.
/// The driver must implement the `FatFsDriver` trait.
/// The driver is placed on the heap using `Box` so that it lives for the lifetime of
/// the program.
#[allow(static_mut_refs)]
pub unsafe fn install(driver: &'static mut (dyn FatFsDriver + 'static)) {
    //let boxed_driver = Box::new(driver);
    DRIVER.replace(driver);
}

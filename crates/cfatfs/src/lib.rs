//! FatFs is a generic FAT/exFAT filesystem module designed for small embedded systems.
//! `fatfs-embedded` is a Rust wrapper of the popular FatFs library from
//! [elm-chan.org](http://elm-chan.org/fsw/ff/00index_e.html).
//! It is based on the R0.15 release.
//!
//! # Goals
//! * Embedded use - This library is `no_std` by default, but is `std` compatible for
//! testing purposes when targeting an OS.
//! * Thread safe - The choice was made to have a dependency on the Embassy
//! framework for concurrency support which is suitable for embedded systems. A global
//! file system mutex is implemented in favor of the `FF_FS_REENTRANT` option, which is
//! more suitable to a Rust implementation.
//! * Portable - Implement the `FatFsDriver` trait to add support for any block device.
//! To support this implementation, `alloc` support is unfortunately required due to the
//! structure of FatFs. A simulated block storage driver implementation is included for
//! test purposes that may be used for reference.
//!
//! ## Drop
//! The decision was made not to implement the `Drop` trait for files or directories.
//! This is because doing so would require acquiring a lock on the file system object,
//! which can easily cause a lockup condition.
//! Files and directories must be manually closed. (The file system object itself is
//! implemented as a static singleton and thus is never dropped.)
//!
//! # FatFs Configuration
//! Most features of FatFs are enabled with a few exceptions:
//! * `FF_USE_FORWARD` is disabled to avoid using additional `unsafe` code.
//! * `FF_CODE_PAGE` is set to 0 and thus must be set via a call to `setcp()`.
//! * `FF_VOLUMES` is currently set to 1 limiting the number of volumes supported to 1.
//! * `FF_MULTI_PARTITION` is not currently supported.
//! * `FF_FS_LOCK` is configured to support 10 simultaneous open files.
//! * An implementation of the `f_printf()` function is not provided.
//!
//! # Features
//! * `chrono` (default) - Enables time support in the library. Access to an RTC may be
//! provided via an implementation of the `FatFsDriver` trait.
//!
//! # Examples
//! A brief example that formats and mounts a simulated drive, writes a string to a file,
//! then reads the data back:
//! ```
//! #[path = "../tests/simulated_driver.rs"]
//! mod simulated_driver;
//!
//! use fatfs_embedded::fatfs::{self, File, FileOptions, FormatOptions};
//! use embassy_futures::block_on;
//!
//! const TEST_STRING: &[u8] = b"Hello world!";
//!
//! //Install a block device driver that implements `FatFsDriver`
//! let driver = simulated_driver::RamBlockStorage::new();
//! block_on(fatfs::diskio::install(driver));
//!
//! //Acquire a lock on the file system.
//! let mut locked_fs = block_on(fatfs::FS.lock());
//!
//! //Format the drive.
//! locked_fs.mkfs("", FormatOptions::FAT32, 0, 0, 0, 0);
//!
//! //Mount the drive.
//! locked_fs.mount();
//!
//! //Create a new file.
//! let mut test_file: File = locked_fs.open("test.txt",
//!     FileOptions::CreateAlways |
//!     FileOptions::Read |
//!     FileOptions::Write).unwrap();
//!
//! //Write a string to the file.
//! locked_fs.write(&mut test_file, TEST_STRING);
//!
//! //Seek back to the beginning of the file.
//! locked_fs.seek(&mut test_file, 0);
//!
//! //Read the string back from the file.
//! let mut read_back: [u8; TEST_STRING.len()] = [0; TEST_STRING.len()];
//! locked_fs.read(&mut test_file, &mut read_back);
//! assert_eq!(TEST_STRING, read_back);
//!
//! //Close the file when done.
//! locked_fs.close(&mut test_file);
//! ```

#![no_std]

use crate::fatfs::{File, FileOptions, FS_SD};

pub mod fatfs {

    /// Block storage I/O objects are located here.
    pub mod diskio;
    pub(crate) mod inc_bindings;

    extern crate alloc;

    use crate::fatfs::inc_bindings::*;
    use alloc::string::String;
    use bitflags::bitflags;
    use core::{ptr, str::FromStr};

    #[cfg(feature = "chrono")]
    use chrono::{Datelike, NaiveDateTime, Timelike};

    #[derive(Debug, PartialEq)]
    pub enum Error {
        DiskError = FRESULT_FR_DISK_ERR as isize,
        IntError = FRESULT_FR_INT_ERR as isize,
        NotReady = FRESULT_FR_NOT_READY as isize,
        NoFile = FRESULT_FR_NO_FILE as isize,
        NoPath = FRESULT_FR_NO_PATH as isize,
        InvalidName = FRESULT_FR_INVALID_NAME as isize,
        Denied = FRESULT_FR_DENIED as isize,
        Exists = FRESULT_FR_EXIST as isize,
        InvalidObject = FRESULT_FR_INVALID_OBJECT as isize,
        WriteProtected = FRESULT_FR_WRITE_PROTECTED as isize,
        InvalidDrive = FRESULT_FR_INVALID_DRIVE as isize,
        NotEnabled = FRESULT_FR_NOT_ENABLED as isize,
        NoFileSystem = FRESULT_FR_NO_FILESYSTEM as isize,
        MkfsAborted = FRESULT_FR_MKFS_ABORTED as isize,
        Timeout = FRESULT_FR_TIMEOUT as isize,
        Locked = FRESULT_FR_LOCKED as isize,
        NotEnoughCore = FRESULT_FR_NOT_ENOUGH_CORE as isize,
        TooManyOpenFiles = FRESULT_FR_TOO_MANY_OPEN_FILES as isize,
        InvalidParameter = FRESULT_FR_INVALID_PARAMETER as isize,
        InternalLogicError,
    }

    impl From<u32> for Error {
        fn from(v: u32) -> Self {
            match v {
                FRESULT_FR_DISK_ERR => (Error::DiskError),
                FRESULT_FR_INT_ERR => (Error::IntError),
                FRESULT_FR_NOT_READY => (Error::NotReady),
                FRESULT_FR_NO_FILE => (Error::NoFile),
                FRESULT_FR_NO_PATH => (Error::NoPath),
                FRESULT_FR_INVALID_NAME => (Error::InvalidName),
                FRESULT_FR_DENIED => (Error::Denied),
                FRESULT_FR_EXIST => (Error::Exists),
                FRESULT_FR_INVALID_OBJECT => (Error::InvalidObject),
                FRESULT_FR_WRITE_PROTECTED => (Error::WriteProtected),
                FRESULT_FR_INVALID_DRIVE => (Error::InvalidDrive),
                FRESULT_FR_NOT_ENABLED => (Error::NotEnabled),
                FRESULT_FR_NO_FILESYSTEM => (Error::NoFileSystem),
                FRESULT_FR_MKFS_ABORTED => (Error::MkfsAborted),
                FRESULT_FR_TIMEOUT => (Error::Timeout),
                FRESULT_FR_LOCKED => (Error::Locked),
                FRESULT_FR_NOT_ENOUGH_CORE => (Error::NotEnoughCore),
                FRESULT_FR_TOO_MANY_OPEN_FILES => (Error::TooManyOpenFiles),
                FRESULT_FR_INVALID_PARAMETER => (Error::InvalidParameter),
                _ => Error::InternalLogicError,
            }
        }
    }

    impl Default for FATFS {
        fn default() -> FATFS {
            FATFS {
                fs_type: Default::default(),
                pdrv: Default::default(),
                ldrv: Default::default(),
                n_fats: Default::default(),
                wflag: Default::default(),
                fsi_flag: Default::default(),
                id: Default::default(),
                n_rootdir: Default::default(),
                csize: Default::default(),
                last_clst: Default::default(),
                free_clst: Default::default(),
                n_fatent: Default::default(),
                fsize: Default::default(),
                volbase: Default::default(),
                fatbase: Default::default(),
                dirbase: Default::default(),
                database: Default::default(),
                winsect: Default::default(),
                win: [0; 512],
                lfnbuf: ptr::null_mut(),
                cdir: Default::default(),
            }
        }
    }

    impl Default for FFOBJID {
        fn default() -> Self {
            Self {
                fs: ptr::null_mut(),
                id: Default::default(),
                attr: Default::default(),
                stat: Default::default(),
                sclust: Default::default(),
                objsize: Default::default(),
                lockid: Default::default(),
            }
        }
    }

    impl Default for FIL {
        fn default() -> Self {
            Self {
                obj: Default::default(),
                flag: Default::default(),
                err: Default::default(),
                fptr: Default::default(),
                clust: Default::default(),
                sect: Default::default(),
                dir_sect: Default::default(),
                dir_ptr: ptr::null_mut(),
                buf: [0; 512],
                cltbl: ptr::null_mut(),
            }
        }
    }

    impl Default for DIR {
        fn default() -> Self {
            Self {
                obj: Default::default(),
                dptr: Default::default(),
                clust: Default::default(),
                sect: Default::default(),
                dir: ptr::null_mut(),
                fn_: Default::default(),
                blk_ofs: Default::default(),
                pat: ptr::null_mut(),
            }
        }
    }

    impl Default for FILINFO {
        fn default() -> Self {
            Self {
                fsize: Default::default(),
                fdate: Default::default(),
                ftime: Default::default(),
                fattrib: Default::default(),
                fname: [0; 256],
                altname: Default::default(),
            }
        }
    }

    bitflags! {
        pub struct FileOptions: u8 {
            const Read = FA_READ as u8;
            const Write = FA_WRITE as u8;
            const OpenExisting = FA_OPEN_EXISTING as u8;
            const CreateNew = FA_CREATE_NEW as u8;
            const CreateAlways = FA_CREATE_ALWAYS as u8;
            const OpenAlways = FA_OPEN_ALWAYS as u8;
            const OpenAppend = FA_OPEN_APPEND as u8;
        }
    }

    bitflags! {
        pub struct FileAttributes: u8 {
            const ReadOnly = AM_RDO as u8;
            const Hidden = AM_HID as u8;
            const System = AM_SYS as u8;
            const Directory = AM_DIR as u8;
            const Archive = AM_ARC as u8;
        }
    }

    bitflags! {
        pub struct FormatOptions: u8 {
            const FAT = FM_FAT as u8;
            const FAT32 = FM_FAT32 as u8;
            const EXFAT = FM_EXFAT as u8;
            const Any = FM_ANY as u8;
        }
    }

    impl FileOptions {
        pub fn as_u8(&self) -> u8 {
            self.bits() as u8
        }
    }

    impl FileAttributes {
        pub fn as_u8(&self) -> u8 {
            self.bits() as u8
        }
    }

    impl FormatOptions {
        pub fn as_u8(&self) -> u8 {
            self.bits() as u8
        }
    }

    pub type FileSystem = RawFileSystem;
    pub type File = FIL;
    pub type Directory = DIR;
    pub type FileInfo = FILINFO;

    /// This is the file system singleton object. Access the file system
    /// API by acquiring a lock on this object.
    pub static mut FS_SD: FileSystem = RawFileSystem {
        fs: FATFS {
            fs_type: 0,
            pdrv: 0,
            ldrv: 0,
            n_fats: 0,
            wflag: 0,
            fsi_flag: 0,
            id: 0,
            n_rootdir: 0,
            csize: 0,
            last_clst: 0,
            free_clst: 0,
            n_fatent: 0,
            fsize: 0,
            volbase: 0,
            fatbase: 0,
            dirbase: 0,
            database: 0,
            winsect: 0,
            win: [0; 512],
            lfnbuf: ptr::null_mut(),
            cdir: 0,
        },
    };
    pub static mut FS_NAND: FileSystem = RawFileSystem {
        fs: FATFS {
            fs_type: 0,
            pdrv: 0,
            ldrv: 0,
            n_fats: 0,
            wflag: 0,
            fsi_flag: 0,
            id: 0,
            n_rootdir: 0,
            csize: 0,
            last_clst: 0,
            free_clst: 0,
            n_fatent: 0,
            fsize: 0,
            volbase: 0,
            fatbase: 0,
            dirbase: 0,
            database: 0,
            winsect: 0,
            win: [0; 512],
            lfnbuf: ptr::null_mut(),
            cdir: 0,
        },
    };

    /// The file system API is located here.
    pub struct RawFileSystem {
        fs: FATFS,
    }

    impl RawFileSystem {
        /// Forces a write of all data to storage. Whether this has any effect depends on the driver implementation.
        pub fn sync(&self, file: &mut File) -> Result<(), Error> {
            let result;
            unsafe {
                result = f_sync(ptr::addr_of_mut!(*file));
            }
            if result == FRESULT_FR_OK {
                return Ok(());
            } else {
                return Err(Error::try_from(result).unwrap());
            }
        }

        /// Find the first item that matches the given pattern.
        /// On success a tuple is returned containing file information and the enclosing directory.
        pub fn findfirst(&self, path: &str, pattern: &str) -> Result<(Directory, FileInfo), Error> {
            let result;
            let mut info: FileInfo = Default::default();
            let mut dir: Directory = Default::default();
            unsafe {
                result = f_findfirst(
                    ptr::addr_of_mut!(dir),
                    ptr::addr_of_mut!(info),
                    path.as_ptr().cast(),
                    pattern.as_ptr().cast(),
                );
            }
            if result == FRESULT_FR_OK {
                return Ok((dir, info));
            } else {
                return Err(Error::try_from(result).unwrap());
            }
        }

        /// Returns the next item that matches a pattern following a call to `findfirst()`.
        pub fn findnext(&self, dir: &mut Directory) -> Result<FileInfo, Error> {
            let result;
            let mut info: FileInfo = Default::default();
            unsafe {
                result = f_findnext(ptr::addr_of_mut!(*dir), ptr::addr_of_mut!(info));
            }
            if result == FRESULT_FR_OK {
                return Ok(info);
            } else {
                return Err(Error::try_from(result).unwrap());
            }
        }

        /// Create a directory at the specified path.
        pub fn mkdir(&self, path: &str) -> Result<(), Error> {
            let result;
            unsafe {
                result = f_mkdir(path.as_ptr().cast());
            }
            if result == FRESULT_FR_OK {
                return Ok(());
            } else {
                return Err(Error::try_from(result).unwrap());
            }
        }

        /// Deletes a file at the specified path.
        pub fn unlink(&self, path: &str) -> Result<(), Error> {
            let result;
            unsafe {
                result = f_unlink(path.as_ptr().cast());
            }
            if result == FRESULT_FR_OK {
                return Ok(());
            } else {
                return Err(Error::try_from(result).unwrap());
            }
        }

        /// Renames a file at the old path to the new path.
        pub fn rename(&self, old_path: &str, new_path: &str) -> Result<(), Error> {
            let result;
            unsafe {
                result = f_rename(old_path.as_ptr().cast(), new_path.as_ptr().cast());
            }
            if result == FRESULT_FR_OK {
                return Ok(());
            } else {
                return Err(Error::try_from(result).unwrap());
            }
        }

        /// Returns information about a file at the given path.
        pub fn stat(&self, path: &str) -> Result<FileInfo, Error> {
            let result;
            let mut info: FileInfo = Default::default();
            unsafe {
                result = f_stat(path.as_ptr().cast(), ptr::addr_of_mut!(info));
            }
            if result == FRESULT_FR_OK {
                return Ok(info);
            } else {
                return Err(Error::try_from(result).unwrap());
            }
        }

        /// Applies the given attributes to the file according to the supplied mask.
        pub fn chmod(
            &self,
            path: &str,
            attr: FileAttributes,
            mask: FileAttributes,
        ) -> Result<(), Error> {
            let result;
            unsafe {
                result = f_chmod(path.as_ptr().cast(), attr.as_u8(), mask.as_u8());
            }
            if result == FRESULT_FR_OK {
                return Ok(());
            } else {
                return Err(Error::try_from(result).unwrap());
            }
        }

        /// Applies a timestamp to the given file.
        #[cfg(feature = "chrono")]
        pub fn utime(&self, path: &str, timestamp: NaiveDateTime) -> Result<(), Error> {
            let result;
            let year = timestamp.year() as u32;
            let month = timestamp.month();
            let day = timestamp.day();
            let hour = timestamp.hour();
            let minute = timestamp.minute();
            let second = timestamp.second();
            let mut info = FileInfo::default();
            info.fdate = (((year - 1980) * 512) | month * 32 | day) as u16;
            info.ftime = (hour * 2048 | minute * 32 | second / 2) as u16;
            unsafe {
                result = f_utime(path.as_ptr().cast(), ptr::addr_of_mut!(info));
            }
            if result == FRESULT_FR_OK {
                return Ok(());
            } else {
                return Err(Error::try_from(result).unwrap());
            }
        }

        /// Change the current directory to the given path.
        pub fn chdir(&self, path: &str) -> Result<(), Error> {
            let result;
            unsafe {
                result = f_chdir(path.as_ptr().cast());
            }
            if result == FRESULT_FR_OK {
                return Ok(());
            } else {
                return Err(Error::try_from(result).unwrap());
            }
        }

        /// Change the current drive.
        pub fn chdrive(&self, path: &str) -> Result<(), Error> {
            let result;
            unsafe {
                result = f_chdrive(path.as_ptr().cast());
            }
            if result == FRESULT_FR_OK {
                return Ok(());
            } else {
                return Err(Error::try_from(result).unwrap());
            }
        }

        /// Retrieves full path name of the current directory of the current drive.
        /// The supplied String buffer must have sufficient capacity to read the entire path.
        pub fn getcwd(&self, buffer: &mut String) -> Result<(), Error> {
            let result;
            unsafe {
                result = f_getcwd(buffer.as_mut_ptr().cast(), buffer.capacity() as u32);
            }
            if result == FRESULT_FR_OK {
                return Ok(());
            } else {
                return Err(Error::try_from(result).unwrap());
            }
        }

        /// Get number of free clusters on the drive.
        pub fn getfree(&self, path: &str) -> Result<u32, Error> {
            let result;
            let mut num_clusters = 0;
            let mut fs_ptr: *mut FATFS = ptr::null_mut();
            unsafe {
                result = f_getfree(
                    path.as_ptr().cast(),
                    ptr::addr_of_mut!(num_clusters),
                    ptr::addr_of_mut!(fs_ptr),
                );
            }
            if result == FRESULT_FR_OK {
                return Ok(num_clusters);
            } else {
                return Err(Error::try_from(result).unwrap());
            }
        }

        /// Get the volume label.
        /// The supplied String buffer must have sufficient capacity to read the entire label.
        pub fn getlabel(&self, path: &str, label: &mut String) -> Result<u32, Error> {
            let result;
            let mut vsn = 0;
            if label.capacity() < 34 {
                //From FATFS documentation, this is the max length required for this parameter.
                return Err(Error::InvalidParameter);
            }
            unsafe {
                result = f_getlabel(
                    path.as_ptr().cast(),
                    label.as_mut_ptr().cast(),
                    ptr::addr_of_mut!(vsn),
                );
            }
            if result == FRESULT_FR_OK {
                return Ok(vsn);
            } else {
                return Err(Error::try_from(result).unwrap());
            }
        }

        /// Set the volume label.
        pub fn setlabel(&self, label: &str) -> Result<(), Error> {
            let result;
            unsafe {
                result = f_setlabel(label.as_ptr().cast());
            }
            if result == FRESULT_FR_OK {
                return Ok(());
            } else {
                return Err(Error::try_from(result).unwrap());
            }
        }

        /// Allocate a contiguous block to the given file.
        pub fn expand(&self, file: &mut File, size: u32) -> Result<(), Error> {
            let result;
            unsafe {
                result = f_expand(ptr::addr_of_mut!(*file), size, 1);
            }
            if result == FRESULT_FR_OK {
                return Ok(());
            } else {
                return Err(Error::try_from(result).unwrap());
            }
        }

        /// Mount the drive.
        pub fn mount(&mut self, drive: &'static core::ffi::CStr) -> Result<(), Error> {
            self.fs = FATFS::default();
            let result;
            unsafe {
                result = f_mount(ptr::addr_of_mut!(self.fs), drive.as_ptr(), 1);
            }
            if result == FRESULT_FR_OK {
                return Ok(());
            } else {
                return Err(Error::try_from(result).unwrap());
            }
        }

        /// Format the drive according to the supplied options.
        pub fn mkfs(
            &self,
            path: &str,
            format: FormatOptions,
            copies: u8,
            alignment: u32,
            au_size: u32,
            root_entries: u32,
        ) -> Result<(), Error> {
            let result;
            let mut work: [u8; FF_MAX_SS as usize] = [0; FF_MAX_SS as usize];
            let parameters = MKFS_PARM {
                fmt: format.as_u8(),
                n_fat: copies,
                align: alignment,
                n_root: root_entries,
                au_size: au_size,
            };
            unsafe {
                result = f_mkfs(
                    path.as_ptr().cast(),
                    ptr::addr_of!(parameters),
                    work.as_mut_ptr().cast(),
                    work.len() as u32,
                );
            }
            if result == FRESULT_FR_OK {
                return Ok(());
            } else {
                return Err(Error::try_from(result).unwrap());
            }
        }

        /// Set the code page.
        pub fn setcp(&self, code_page: u16) -> Result<(), Error> {
            let result;
            unsafe {
                result = f_setcp(code_page);
            }
            if result == FRESULT_FR_OK {
                return Ok(());
            } else {
                return Err(Error::try_from(result).unwrap());
            }
        }

        /// Write a character to the file.
        pub fn putc(&self, file: &mut File, char: u8) -> Result<i32, Error> {
            let result;
            unsafe {
                result = f_putc(char as TCHAR, ptr::addr_of_mut!(*file));
            }
            if result >= 0 {
                return Ok(result);
            } else {
                return Err(Error::Denied);
            }
        }

        /// Write a string to the file.
        pub fn puts(&self, file: &mut File, string: &str) -> Result<i32, Error> {
            let result;
            unsafe {
                result = f_puts(string.as_ptr().cast(), ptr::addr_of_mut!(*file));
            }
            if result >= 0 {
                Ok(result)
            } else {
                Err(Error::Denied)
            }
        }

        /// Get a string from the file.
        /// The capacity of the supplied String buffer determines the maximum length of data read.
        pub fn gets(&self, file: &mut File, buffer: &mut String) -> Result<(), Error> {
            let result;
            unsafe {
                result = f_gets(
                    buffer.as_mut_ptr().cast(),
                    buffer.capacity() as i32,
                    ptr::addr_of_mut!(*file),
                );
            }
            if result != ptr::null_mut() {
                Ok(())
            } else {
                Err(Error::Denied)
            }
        }

        /// Unmount the drive at the supplied path.
        pub fn unmount(&self, path: &str) -> Result<(), Error> {
            let result;
            unsafe {
                result = f_mount(ptr::null_mut(), path.as_ptr().cast(), 0);
            }
            if result == FRESULT_FR_OK {
                Ok(())
            } else {
                Err(Error::from(result))
            }
        }
    }
}
use crate::fatfs::inc_bindings::*;
use crate::fatfs::*;
use core::ptr::addr_of_mut;
use core::str::FromStr;
extern crate alloc;

/// Opens the file at the given path in the given mode. FileOption flags may be OR'd together.
pub fn open(
    path: &mut alloc::string::String,
    mode: FileOptions,
) -> Result<File, crate::fatfs::Error> {
    let mut file = Default::default();
    path.push('\0');
    let result = unsafe { f_open(addr_of_mut!(file), path.as_ptr().cast(), mode.as_u8()) };
    path.pop();
    if result == FRESULT_FR_OK {
        Ok(file)
    } else {
        Err(Error::from(result))
    }
}
/// Opens a directory. On success, the Directory object is returned.
pub fn opendir(path: &mut alloc::string::String) -> Result<Directory, Error> {
    let mut dir = Default::default();
    path.push('\0');
    let result = unsafe { f_opendir(addr_of_mut!(dir), path.as_ptr().cast()) };
    path.pop();
    match result {
        FRESULT_FR_OK => Ok(dir),
        error => Err(Error::from(error)),
    }
}
/// Move to an offset in the given file. This represents the location within the file for where data is read or written.
pub fn seek(file: &mut File, offset: u32) -> Result<(), Error> {
    match unsafe { f_lseek(addr_of_mut!(*file), offset) } {
        FRESULT_FR_OK => Ok(()),
        error => Err(Error::from(error)),
    }
}

/// Read data from the given file. The length of the provided buffer determines the length of data read.
pub fn read(file: &mut File, buffer: &mut [u8]) -> Result<u32, Error> {
    let mut bytes_read: UINT = 0;
    let result = unsafe {
        f_read(
            addr_of_mut!(*file),
            buffer.as_mut_ptr().cast(),
            buffer.len() as u32,
            addr_of_mut!(bytes_read),
        )
    };
    if result == FRESULT_FR_OK {
        Ok(bytes_read)
    } else {
        Err(Error::from(result))
    }
}

/// Closes the given file.
fn close(file: &mut File) -> Result<(), Error> {
    match unsafe { f_close(addr_of_mut!(*file)) } {
        FRESULT_FR_OK => Ok(()),
        error => Err(Error::from(error)),
    }
}

/// Write data to the given file. The length of the provided buffer determines the length of data written.
pub fn write(file: &mut File, buffer: &[u8]) -> Result<u32, Error> {
    let mut bytes_written: UINT = 0;
    let result = unsafe {
        f_write(
            addr_of_mut!(*file),
            buffer.as_ptr().cast(),
            buffer.len() as u32,
            addr_of_mut!(bytes_written),
        )
    };
    if result == FRESULT_FR_OK {
        Ok(bytes_written)
    } else {
        Err(Error::from(result))
    }
}
/// Closes the given directory.
fn closedir(dir: &mut Directory) -> Result<(), Error> {
    let result;
    unsafe {
        result = f_closedir(addr_of_mut!(*dir));
    }
    if result == FRESULT_FR_OK {
        Ok(())
    } else {
        Err(Error::from(result))
    }
}
/// Truncates the given file to the current position.
pub fn truncate(file: &mut File) -> Result<(), Error> {
    let result = unsafe { f_truncate(addr_of_mut!(*file)) };
    if result == FRESULT_FR_OK {
        Ok(())
    } else {
        Err(Error::from(result))
    }
}

/// Gets information about items within the given directory.
/// Each call to this function returns the next item in sequence, until a null string is returned.
pub fn readdir(dir: &mut Directory) -> Result<FileInfo, Error> {
    let result;
    let mut info: FileInfo = Default::default();
    unsafe {
        result = f_readdir(addr_of_mut!(*dir), addr_of_mut!(info));
    }
    if result == FRESULT_FR_OK {
        Ok(info)
    } else {
        Err(Error::from(result))
    }
}
pub fn size(file: &mut File) -> u32 {
    file.obj.objsize
}

impl Drop for File {
    fn drop(&mut self) {
        let _ = crate::close(self);
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = crate::closedir(self);
    }
}

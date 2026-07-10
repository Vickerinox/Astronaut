/* 
MIT License

Copyright (c) 2024 Caleb Garrett

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.
*/

#![no_std]

use crate::fatfs::{File, FileOptions};

pub mod fatfs {

    /// Block storage I/O objects are located here.
    pub mod diskio;
    pub(crate) mod inc_bindings;

    extern crate alloc;

    use crate::fatfs::inc_bindings::*;
    use alloc::string::String;
    use bitflags::bitflags;
    use core::ptr;

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
                FRESULT_FR_DISK_ERR => Error::DiskError,
                FRESULT_FR_INT_ERR => Error::IntError,
                FRESULT_FR_NOT_READY => Error::NotReady,
                FRESULT_FR_NO_FILE => Error::NoFile,
                FRESULT_FR_NO_PATH => Error::NoPath,
                FRESULT_FR_INVALID_NAME => Error::InvalidName,
                FRESULT_FR_DENIED => Error::Denied,
                FRESULT_FR_EXIST => Error::Exists,
                FRESULT_FR_INVALID_OBJECT => Error::InvalidObject,
                FRESULT_FR_WRITE_PROTECTED => Error::WriteProtected,
                FRESULT_FR_INVALID_DRIVE => Error::InvalidDrive,
                FRESULT_FR_NOT_ENABLED => Error::NotEnabled,
                FRESULT_FR_NO_FILESYSTEM => Error::NoFileSystem,
                FRESULT_FR_MKFS_ABORTED => Error::MkfsAborted,
                FRESULT_FR_TIMEOUT => Error::Timeout,
                FRESULT_FR_LOCKED => Error::Locked,
                FRESULT_FR_NOT_ENOUGH_CORE => Error::NotEnoughCore,
                FRESULT_FR_INVALID_PARAMETER => Error::InvalidParameter,
                FRESULT_FR_TOO_MANY_OPEN_FILES => Error::TooManyOpenFiles,
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

    /// The file system API is located here.
    pub struct RawFileSystem {
        fs: FATFS,
    }

    impl RawFileSystem {
        pub const fn uninit() -> Self {
            RawFileSystem {
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
            }
        }
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

const UC437: &[u16] = &[
    /*  CP437(U.S.) to Unicode conversion table */
    0x00C7, 0x00FC, 0x00E9, 0x00E2, 0x00E4, 0x00E0, 0x00E5, 0x00E7, 0x00EA, 0x00EB, 0x00E8, 0x00EF,
    0x00EE, 0x00EC, 0x00C4, 0x00C5, 0x00C9, 0x00E6, 0x00C6, 0x00F4, 0x00F6, 0x00F2, 0x00FB, 0x00F9,
    0x00FF, 0x00D6, 0x00DC, 0x00A2, 0x00A3, 0x00A5, 0x20A7, 0x0192, 0x00E1, 0x00ED, 0x00F3, 0x00FA,
    0x00F1, 0x00D1, 0x00AA, 0x00BA, 0x00BF, 0x2310, 0x00AC, 0x00BD, 0x00BC, 0x00A1, 0x00AB, 0x00BB,
    0x2591, 0x2592, 0x2593, 0x2502, 0x2524, 0x2561, 0x2562, 0x2556, 0x2555, 0x2563, 0x2551, 0x2557,
    0x255D, 0x255C, 0x255B, 0x2510, 0x2514, 0x2534, 0x252C, 0x251C, 0x2500, 0x253C, 0x255E, 0x255F,
    0x255A, 0x2554, 0x2569, 0x2566, 0x2560, 0x2550, 0x256C, 0x2567, 0x2568, 0x2564, 0x2565, 0x2559,
    0x2558, 0x2552, 0x2553, 0x256B, 0x256A, 0x2518, 0x250C, 0x2588, 0x2584, 0x258C, 0x2590, 0x2580,
    0x03B1, 0x00DF, 0x0393, 0x03C0, 0x03A3, 0x03C3, 0x00B5, 0x03C4, 0x03A6, 0x0398, 0x03A9, 0x03B4,
    0x221E, 0x03C6, 0x03B5, 0x2229, 0x2261, 0x00B1, 0x2265, 0x2264, 0x2320, 0x2321, 0x00F7, 0x2248,
    0x00B0, 0x2219, 0x00B7, 0x221A, 0x207F, 0x00B2, 0x25A0, 0x00A0,
];

#[no_mangle]
pub unsafe extern "C" fn ff_uni2oem(uni: u32, cp: u16) -> u32 {
    if uni < 0x80 {
        /* ASCII? */
        uni
    } else {
        /* Non-ASCII */
        if uni < 0x10000 && cp == 437 {
            /* Is it in BMP and valid code page? */
            let mut c = 0;
            while c < 0x80 && Some(uni as u16) != UC437.get(c as usize).copied() {
                c = (c + 0x80) & 0xFF;
                c += 1;
            }
            c
        } else {
            0
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn ff_oem2uni(oem: u32, cp: u16) -> u32 {
    if oem < 0x80 {
        /* ASCII? */
        oem
    } else if cp == 437 {
        /* Extended char */
        /* Is it a valid code page? */
        if oem < 0x100 {
            UC437.get((oem - 0x80) as usize).copied().unwrap_or(0) as u32
        } else {
            0
        } 
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn ff_wtoupper(mut uni: u32) -> u32 {
    const CVT1: &[u16] = &[
        /* Compressed up conversion table for U+0000 - U+0FFF */
        /* Basic Latin */
        0x0061, 0x031A, /* Latin-1 Supplement */
        0x00E0, 0x0317, 0x00F8, 0x0307, 0x00FF, 0x0001, 0x0178, /* Latin Extended-A */
        0x0100, 0x0130, 0x0132, 0x0106, 0x0139, 0x0110, 0x014A, 0x012E, 0x0179, 0x0106,
        /* Latin Extended-B */
        0x0180, 0x004D, 0x0243, 0x0181, 0x0182, 0x0182, 0x0184, 0x0184, 0x0186, 0x0187, 0x0187,
        0x0189, 0x018A, 0x018B, 0x018B, 0x018D, 0x018E, 0x018F, 0x0190, 0x0191, 0x0191, 0x0193,
        0x0194, 0x01F6, 0x0196, 0x0197, 0x0198, 0x0198, 0x023D, 0x019B, 0x019C, 0x019D, 0x0220,
        0x019F, 0x01A0, 0x01A0, 0x01A2, 0x01A2, 0x01A4, 0x01A4, 0x01A6, 0x01A7, 0x01A7, 0x01A9,
        0x01AA, 0x01AB, 0x01AC, 0x01AC, 0x01AE, 0x01AF, 0x01AF, 0x01B1, 0x01B2, 0x01B3, 0x01B3,
        0x01B5, 0x01B5, 0x01B7, 0x01B8, 0x01B8, 0x01BA, 0x01BB, 0x01BC, 0x01BC, 0x01BE, 0x01F7,
        0x01C0, 0x01C1, 0x01C2, 0x01C3, 0x01C4, 0x01C5, 0x01C4, 0x01C7, 0x01C8, 0x01C7, 0x01CA,
        0x01CB, 0x01CA, 0x01CD, 0x0110, 0x01DD, 0x0001, 0x018E, 0x01DE, 0x0112, 0x01F3, 0x0003,
        0x01F1, 0x01F4, 0x01F4, 0x01F8, 0x0128, 0x0222, 0x0112, 0x023A, 0x0009, 0x2C65, 0x023B,
        0x023B, 0x023D, 0x2C66, 0x023F, 0x0240, 0x0241, 0x0241, 0x0246, 0x010A,
        /* IPA Extensions */
        0x0253, 0x0040, 0x0181, 0x0186, 0x0255, 0x0189, 0x018A, 0x0258, 0x018F, 0x025A, 0x0190,
        0x025C, 0x025D, 0x025E, 0x025F, 0x0193, 0x0261, 0x0262, 0x0194, 0x0264, 0x0265, 0x0266,
        0x0267, 0x0197, 0x0196, 0x026A, 0x2C62, 0x026C, 0x026D, 0x026E, 0x019C, 0x0270, 0x0271,
        0x019D, 0x0273, 0x0274, 0x019F, 0x0276, 0x0277, 0x0278, 0x0279, 0x027A, 0x027B, 0x027C,
        0x2C64, 0x027E, 0x027F, 0x01A6, 0x0281, 0x0282, 0x01A9, 0x0284, 0x0285, 0x0286, 0x0287,
        0x01AE, 0x0244, 0x01B1, 0x01B2, 0x0245, 0x028D, 0x028E, 0x028F, 0x0290, 0x0291, 0x01B7,
        /* Greek, Coptic */
        0x037B, 0x0003, 0x03FD, 0x03FE, 0x03FF, 0x03AC, 0x0004, 0x0386, 0x0388, 0x0389, 0x038A,
        0x03B1, 0x0311, 0x03C2, 0x0002, 0x03A3, 0x03A3, 0x03C4, 0x0308, 0x03CC, 0x0003, 0x038C,
        0x038E, 0x038F, 0x03D8, 0x0118, 0x03F2, 0x000A, 0x03F9, 0x03F3, 0x03F4, 0x03F5, 0x03F6,
        0x03F7, 0x03F7, 0x03F9, 0x03FA, 0x03FA, /* Cyrillic */
        0x0430, 0x0320, 0x0450, 0x0710, 0x0460, 0x0122, 0x048A, 0x0136, 0x04C1, 0x010E, 0x04CF,
        0x0001, 0x04C0, 0x04D0, 0x0144, /* Armenian */
        0x0561, 0x0426,
    ];
    const CVT2: &[u16] = &[
        /* Compressed up conversion table for U+1000 - U+FFFF */
        /* Phonetic Extensions */
        0x1D7D, 0x0001, 0x2C63, /* Latin Extended Additional */
        0x1E00, 0x0196, 0x1EA0, 0x015A, /* Greek Extended */
        0x1F00, 0x0608, 0x1F10, 0x0606, 0x1F20, 0x0608, 0x1F30, 0x0608, 0x1F40, 0x0606, 0x1F51,
        0x0007, 0x1F59, 0x1F52, 0x1F5B, 0x1F54, 0x1F5D, 0x1F56, 0x1F5F, 0x1F60, 0x0608, 0x1F70,
        0x000E, 0x1FBA, 0x1FBB, 0x1FC8, 0x1FC9, 0x1FCA, 0x1FCB, 0x1FDA, 0x1FDB, 0x1FF8, 0x1FF9,
        0x1FEA, 0x1FEB, 0x1FFA, 0x1FFB, 0x1F80, 0x0608, 0x1F90, 0x0608, 0x1FA0, 0x0608, 0x1FB0,
        0x0004, 0x1FB8, 0x1FB9, 0x1FB2, 0x1FBC, 0x1FCC, 0x0001, 0x1FC3, 0x1FD0, 0x0602, 0x1FE0,
        0x0602, 0x1FE5, 0x0001, 0x1FEC, 0x1FF3, 0x0001, 0x1FFC,
        /* Letterlike Symbols */
        0x214E, 0x0001, 0x2132, /* Number forms */
        0x2170, 0x0210, 0x2184, 0x0001, 0x2183, /* Enclosed Alphanumerics */
        0x24D0, 0x051A, 0x2C30, 0x042F, /* Latin Extended-C */
        0x2C60, 0x0102, 0x2C67, 0x0106, 0x2C75, 0x0102, /* Coptic */
        0x2C80, 0x0164, /* Georgian Supplement */
        0x2D00, 0x0826, /* Full-width */
        0xFF41, 0x031A,
    ];

    if uni < 0x10000 {
        /* Is it in BMP? */
        let uc = uni as u16;
        let mut p = if uc < 0x1000 { CVT1 } else { CVT2 };
        loop {
            let Some(([bc], rem)) = p.split_first_chunk() else {
                break;
            };
            p = rem;
            if uc < *bc {
                break;
            };
            let Some(([nc], rem)) = p.split_first_chunk() else {
                break;
            };
            p = rem;
            let [nc, cmd] = nc.to_le_bytes();

            if uc < bc + nc as u16 {
                /* In the block? */
                match cmd {
                    0 => return (p[(uc - bc) as usize]) as u32,
                    1 => return (uc - (uc - bc) & 1) as u32, /* Case pairs */
                    2 => return uc as u32 - 16,              /* Shift -16 */
                    3 => return uc as u32 - 32,              /* Shift -32 */
                    4 => return uc as u32 - 48,              /* Shift -48 */
                    5 => return uc as u32 - 26,              /* Shift -26 */
                    6 => return uc as u32 + 8,               /* Shift +8 */
                    7 => return uc as u32 - 80,              /* Shift -80 */
                    8 => return uc as u32 - 0x1C60,          /* Shift -0x1C60 */
                    _ => return uc as u32,
                }
            }
            if cmd == 0 {
                let Some(r) = p.get(nc as usize..) else { return uc as u32};
                p = r;
            } /* Skip table if needed */
        }
        uc as u32
    } else {
        uni
    }
}

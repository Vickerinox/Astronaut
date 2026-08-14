// SPDX-FileCopyrightText: 2026 Viktor Karlsson <viktor@koda.re>
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::errors::{CompileError, FileOpError, NANDInjectError};
use core::array;
use fatfs_embedded::fatfs::diskio::DiskResult;
use fatfs_embedded::fatfs::{FileAttributes, FileOptions};
use log::debug;
use mbr::ByteDecode;
use nandcursor::{NandSectorCursor, NandWrapper};
use sha1::{Digest, Sha1};
use std::{fs, vec};
use std::{
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
};
//pub mod aes_ecb;
pub mod mbr;
pub mod nandcursor;

const HWINFO_PATH: &str = "/sys/HWINFO_S.dat";
const REGULAR_TMD_LEN: usize = 520;

fn open_main_twl(nand_image: &mut [u8]) -> Result<NativeFatFsDriver<&mut [u8]>, NANDInjectError> {
    let mut nocash_footer = &nand_image[(nand_image.len() - 64)..];
    if &nocash_footer[0..16] != b"DSi eMMC CID/CPU" {
        nocash_footer = &nand_image[0xFF800..][..64];
        if &nocash_footer[0..16] != b"DSi eMMC CID/CPU" {
            return Err(NANDInjectError::MissingFooter);
        }
    }

    const KEY_SCRAMBLE: u128 = 0xFFFEFB4E_29590258_2A680F5F_1A4F3E79;
    const KEY_X_SEED: u128 = 0x00000000_E65B601D_24EE6906_00000000;
    const CONSOLE_ID_SEQ: [usize; 16] = [0, 1, 2, 3, 0, 1, 2, 3, 4, 5, 6, 7, 4, 5, 6, 7];

    let cid: [u8; 16] = array::from_fn(|i| nocash_footer[i + 0x10]);
    let console_id: [u8; 8] = array::from_fn(|i| nocash_footer[i + 0x20]);

    let ctr = {
        let mut hasher = Sha1::new();
        hasher.update(cid);
        let result = hasher.finalize();
        u128::from_le_bytes(array::from_fn(|i| result[i]))
    };

    let key = {
        let key_x = u128::from_le_bytes(CONSOLE_ID_SEQ.map(|i| console_id[i])) ^ KEY_X_SEED;
        let key_y = 0xE1A00005_202DDD1D_BD4DC4D3_0AB9DC76;
        (key_x ^ key_y).wrapping_add(KEY_SCRAMBLE).rotate_left(42)
    };
    let mut reader = NandSectorCursor::new(
        NandWrapper::new(&mut nand_image[..512]),
        [0u8; 512],
        ctr,
        key,
    );

    let mbr = mbr::MBR::from_reads(&mut reader).map_err(NANDInjectError::MBR)?;
    drop(reader);

    let start = (mbr.partitions[0].lba * 512) as usize;
    let end = start + (mbr.partitions[0].sector_count * 512) as usize;
    let ctr = ctr + (start as u128 >> 4);

    let reader = NandSectorCursor::new(
        NandWrapper::new(&mut nand_image[start..end]),
        [0u8; 512],
        ctr,
        key,
    );

    let fs = NativeFatFsDriver { nand: reader };
    Ok(fs)
}

pub struct NativeFatFsDriver<T: AsMut<[u8]>> {
    nand: NandSectorCursor<[u8; 512], NandWrapper<T, 9>, 9>,
}

impl<T: AsMut<[u8]>> fatfs_embedded::fatfs::diskio::FatFsDriver for NativeFatFsDriver<T> {
    fn disk_status(&mut self, drive: u8) -> u8 {
        match drive {
            1 => 1,
            2 => 0,
            _ => 2,
        }
    }

    fn disk_initialize(&mut self, drive: u8) -> u8 {
        match drive {
            1 => 1,
            2 => 0,
            _ => 2,
        }
    }

    fn disk_read(&mut self, drive: u8, buffer: &mut [u8], sector: u32) -> DiskResult {
        match drive {
            1 => DiskResult::NotReady,
            2 => {
                if self
                    .nand
                    .seek(SeekFrom::Start((sector as u64) << 9))
                    .is_err()
                {
                    return DiskResult::Error;
                }
                match self.nand.read_exact(buffer) {
                    Ok(()) => DiskResult::Ok,
                    Err(_) => DiskResult::Error,
                }
            }
            _ => DiskResult::ParameterError,
        }
    }

    fn disk_write(&mut self, drive: u8, buffer: &[u8], sector: u32) -> DiskResult {
        match drive {
            1 => DiskResult::NotReady,
            2 => {
                if self
                    .nand
                    .seek(SeekFrom::Start((sector as u64) << 9))
                    .is_err()
                {
                    return DiskResult::Error;
                }
                match self.nand.write_all(buffer) {
                    Ok(()) => DiskResult::Ok,
                    Err(_) => DiskResult::Error,
                }
            }
            _ => DiskResult::ParameterError,
        }
    }

    fn disk_ioctl(&mut self, data: &mut fatfs_embedded::fatfs::diskio::IoctlCommand) -> DiskResult {
        match data {
            fatfs_embedded::fatfs::diskio::IoctlCommand::CtrlSync(()) => {
                let _ = self.nand.flush();
                DiskResult::Ok
            }
            _ => DiskResult::ParameterError,
        }
    }
}

static mut FATFS_DRIVER: std::mem::MaybeUninit<NativeFatFsDriver<&mut [u8]>> =
    std::mem::MaybeUninit::uninit();
static mut MMC_IMAGE_BUFFER: &mut [u8; 1024 * 1024 * 256] = &mut [0; _];
static mut NAND_WORK_AREA: fatfs_embedded::fatfs::RawFileSystem =
    fatfs_embedded::fatfs::RawFileSystem::uninit();

pub fn write_tmd_to_image(mmc_path: impl AsRef<Path>, tmd: &[u8]) -> Result<(), CompileError> {
    debug!("Selected NAND image: {:?}", mmc_path.as_ref());
    debug!("Loading NAND Image... ");
    let mmc_image = fs::read(&mmc_path).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => NANDInjectError::ImageNotFound(e),
        _ => NANDInjectError::ReadingImage(e),
    })?;
    let len = mmc_image.len();

    #[allow(static_mut_refs)]
    unsafe {
        let nand_root = std::ffi::CStr::from_bytes_with_nul_unchecked(b"nand:/\0");
        MMC_IMAGE_BUFFER[..len].copy_from_slice(&mmc_image);

        debug!("Mounting TWL_MAIN... ");
        let fs = open_main_twl(&mut MMC_IMAGE_BUFFER[..len])?;

        let driver = FATFS_DRIVER.write(fs);

        fatfs_embedded::fatfs::diskio::install(driver);
        NAND_WORK_AREA
            .mount(nand_root)
            .map_err(|e| NANDInjectError::FileSystemCreation(e))?;
    }

    debug!("Inspecting HWINFO_S.dat... ");
    let tid = {
        let mut hwinfo_file =
            fatfs_embedded::open(&mut std::format!("nand:{HWINFO_PATH}"), FileOptions::Read)
                .map_err(|e| NANDInjectError::HWINFONotFound(e))?;
        fatfs_embedded::seek(&mut hwinfo_file, 0xA0).map_err(|e| NANDInjectError::MMCSeek(e))?;
        let mut tid_buffer = [0u8; 4];
        if fatfs_embedded::read(&mut hwinfo_file, &mut tid_buffer)
            .map_err(|e| NANDInjectError::MMCRead(FileOpError::Fatfs(e)))?
            != tid_buffer.len() as u32
        {
            return Err(NANDInjectError::MMCRead(FileOpError::CutShort).into());
        }
        u32::from_le_bytes(tid_buffer)
    };

    let mut tmd_path = std::format!("nand:/title/00030017/{tid:08x}/content/title.tmd");

    fatfs_embedded::chmod(
        &mut tmd_path,
        FileAttributes::empty(),
        FileAttributes::ReadOnly,
    )
    .unwrap();

    debug!("Opening Title.TMD... ");
    let mut tmd_file = fatfs_embedded::open(&mut tmd_path, FileOptions::Write)
        .map_err(|e| NANDInjectError::TMDFileMissing(e))?;
    fatfs_embedded::seek(&mut tmd_file, REGULAR_TMD_LEN as u32)
        .map_err(|e| NANDInjectError::MMCSeek(e))?;

    debug!("Modifying Title.TMD... ");
    if fatfs_embedded::write(&mut tmd_file, &tmd[REGULAR_TMD_LEN..])
        .map_err(|e| NANDInjectError::MMCWrite(FileOpError::Fatfs(e)))?
        != tmd[REGULAR_TMD_LEN..].len() as u32
    {
        return Err(NANDInjectError::MMCWrite(FileOpError::CutShort).into());
    }
    drop(tmd_file);

    fatfs_embedded::chmod(
        &mut tmd_path,
        FileAttributes::ReadOnly,
        FileAttributes::ReadOnly,
    )
    .unwrap();

    debug!("Verifying Title.TMD... ");

    let mut tmd_file = fatfs_embedded::open(&mut tmd_path, FileOptions::Read)
        .map_err(|_| NANDInjectError::TMDFileVerification)?;
    let size = fatfs_embedded::size(&mut tmd_file) - REGULAR_TMD_LEN as u32;
    let mut buffer = vec![0u8; size as usize];
    fatfs_embedded::seek(&mut tmd_file, REGULAR_TMD_LEN as u32)
        .map_err(|_| NANDInjectError::TMDFileVerification)?;
    if fatfs_embedded::read(&mut tmd_file, &mut buffer)
        .map_err(|_| NANDInjectError::TMDFileVerification)?
        != buffer.len() as u32
    {
        return Err(NANDInjectError::TMDFileVerification.into());
    }
    //assert!(&buffer == &tmd[REGULAR_TMD_LEN..]);
    drop(tmd_file);

    debug!("Writing new MMC Image... ");
    unsafe {
        std::fs::write(&mmc_path, &MMC_IMAGE_BUFFER[..len]).unwrap();
    }
    Ok(())
}

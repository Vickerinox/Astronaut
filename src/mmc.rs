use core::array;
use fatfs::Error as FatFsError;
use fatfs::{FileSystem, FsOptions, StdIoWrapper};
use mbr::ByteDecode;
use nandcursor::{NandSectorCursor, NandWrapper};
use sha1::{Digest, Sha1};
use std::{
    fs::OpenOptions,
    io::{Read, Write},
    path::Path,
};

use crate::errors::{CompileError, TMDCompileError};
pub mod aes_ecb;
pub mod mbr;
pub mod nandcursor;

const TMD_PATH: &str = "/title/00030017/484e4150/content/title.tmd";
fn open_main_twl<'a>(
    nand_image: &'a mut [u8],
) -> Result<
    FileSystem<StdIoWrapper<NandSectorCursor<[u8; 512], NandWrapper<&'a mut [u8], 9>>>>,
    TMDCompileError,
> {
    let nocash_footer = &nand_image[(nand_image.len() - 64)..];
    if &nocash_footer[0..16] != b"DSi eMMC CID/CPU" {
        return Err(TMDCompileError::MissingFooter);
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
    //println!("{ctr:x?} {key:x?}");
    let mut reader = NandSectorCursor::new(
        NandWrapper::new(&mut nand_image[..512]),
        [0u8; 512],
        ctr,
        key,
    );

    let mbr = mbr::MBR::from_reads(&mut reader).map_err(|e| TMDCompileError::MBR(e))?;
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
    let fs = FileSystem::new(reader, FsOptions::new())
        .map_err(|e| TMDCompileError::FileSystemCreation(e))?;
    Ok(fs)
}

pub fn write_tmd_to_image(mmc_path: impl AsRef<Path>, tmd: &[u8]) -> Result<(), CompileError> {
    print!("Loading MMC Image... ");
    let mut mmc_image = {
        let mut buffer = Vec::new();
        OpenOptions::new()
            .read(true)
            .write(false)
            .open(mmc_path.as_ref())
            .map_err(|e| TMDCompileError::MMCNotFound(e))?
            .read_to_end(&mut buffer)
            .map_err(|e| TMDCompileError::MMCRead(e))?;
        buffer
    };
    println!("Done.");
    print!("Loading TMD file... ");
    let tmd_file = tmd;
    println!("Done.");

    print!("Mounting TWL_MAIN... ");
    let fs = open_main_twl(&mut mmc_image)?;
    println!("Done.");

    print!("Modifying Title.TMD... ");
    let root = fs.root_dir();
    let mut file = root
        .open_file(TMD_PATH)
        .map_err(|e| TMDCompileError::FileNotFound {
            source: e,
            path: TMD_PATH.to_owned(),
        })?;
    file.write_all(&tmd_file)
        .map_err(|e| TMDCompileError::Fatfs(FatFsError::Io(e)))?;
    file.truncate().map_err(|e| TMDCompileError::Fatfs(e))?;
    println!("Done.");
    drop(file);
    let mut file = root
        .open_file(TMD_PATH)
        .map_err(|e| TMDCompileError::Fatfs(e))?;

    let mut vec = vec![0u8; tmd_file.len()];
    file.read_exact(&mut vec)
        .map_err(|e| TMDCompileError::Fatfs(FatFsError::Io(e)))?;
    //verify the file
    print!("Verifying TMD... ");
    if &vec == &tmd_file {
        drop(root);
        drop(file);
        println!("Success!");
        print!("Unmounting TWL_MAIN... ");
        fs.unmount().map_err(|e| TMDCompileError::Fatfs(e))?;
        println!("Done.");

        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(mmc_path.as_ref())
            .map_err(|e| TMDCompileError::Fatfs(FatFsError::Io(e)))?;
        assert_eq!(
            file.metadata()
                .map_err(|e| TMDCompileError::Fatfs(FatFsError::Io(e)))?
                .len(),
            mmc_image.len() as u64
        );
        print!("Rewriting NAND image... ");
        file.write_all(&mmc_image).unwrap();
        println!("Finished.");
        Ok(())
    } else {
        println!("Failed, aborting...");
        let diff = hexify::format_hex_dump_comparison_width(&vec, &tmd_file, 16);
        eprintln!("{}", diff);
        Err(CompileError::TMD(TMDCompileError::FileVerification))
    }
}

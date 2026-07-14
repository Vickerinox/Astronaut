#![feature(array_try_from_fn)]
use std::{
    env::{self},
    fs,
    path::PathBuf,
    process::exit,
};

use clap::Parser;
use elf::{endian::AnyEndian, ElfBytes};
//use rfd::FileDialog;
use log::{debug, error, info};

use self::errors::{BuildError, CompileError, Crate, TMDCompileError};
mod build;
mod errors;
mod mmc;
mod testing;

pub struct ElfStat {
    entry_point: u32,
    size: u32,
}
fn inject_elf(
    elf_file_path: &PathBuf,
    memory: &mut [u8],
    start_addr: usize,
) -> Result<ElfStat, CompileError> {
    info!("SELECTED ELF: {:?}", &elf_file_path);
    let file = fs::read(elf_file_path).map_err(|e| CompileError::ElfNotFound(e))?;
    let parse = ElfBytes::<AnyEndian>::minimal_parse(&file[..])
        .map_err(|e| CompileError::ElfParseError(e))?;
    let entrypoint = parse.ehdr.e_entry;

    let Some(segments) = parse.segments() else {
        return Err(CompileError::ElfMissingSegments);
    };
    let entry_point = entrypoint - start_addr as u64;
    let entry_value = (entrypoint as u32) + 4;
    info!(
        "Elf entrypoint: {}, file offset: {:x}, address: {:x}",
        entrypoint, entry_point, entry_value
    );
    let mut end = 0;
    for segment in segments.iter().filter(|f| f.p_type == 1 && f.p_filesz != 0) {
        let file_offset_start = (segment.p_vaddr as i64) - (start_addr as i64);
        let file_offset_end = file_offset_start + segment.p_filesz as i64;
        if file_offset_start.is_negative() {
            continue;
        }
        let data = parse
            .segment_data(&segment)
            .map_err(|e| CompileError::ElfSegmentError(e))?;
        let file_range = (file_offset_start as usize)..(file_offset_end as usize);
        end = file_offset_end.max(end);
        debug!(
            "Processing segment '{:x?}': {} bytes, file start: 0x{:x?}, file end: 0x{:x?}",
            segment.p_flags, segment.p_filesz, file_offset_start, file_offset_end
        );
        memory[file_range].copy_from_slice(data);
    }
    Ok(ElfStat {
        entry_point: entry_value,
        size: end as u32,
    })
}
fn construct_installer_rom(arm9: PathBuf, arm7: PathBuf) -> Result<Vec<u8>, BuildError> {
    let mut rom = vec![0u8; 0x80000];
    let mut header = common::bootstrap::TWLHeader::new();

    let elf_file_path = &arm9;

    const MAGIC_START_POINT_ARM9: usize = 0x02010000;
    const MAGIC_START_POINT_ARM7: usize = 0x02300000;

    let ElfStat { entry_point, size } =
        inject_elf(&arm9, &mut rom[0x4000..], MAGIC_START_POINT_ARM9)
            .map_err(|e| Crate::Arm9Installer.err()(e))?;

    header.head.arm9_load = (MAGIC_START_POINT_ARM9) as u32;
    header.head.arm9_size = size;
    header.head.arm9_entry = entry_point;
    header.head.arm9_offset = 0x4000;

    let arm7_offset = 0x5000 + (size & !0xFFF);

    let ElfStat { entry_point, size } = inject_elf(
        &arm7,
        &mut rom[(arm7_offset as usize)..],
        MAGIC_START_POINT_ARM7,
    )
    .map_err(|e| Crate::Arm7Installer.err()(e))?;

    header.head.arm7_load = (MAGIC_START_POINT_ARM7) as u32;
    header.head.arm7_size = size;
    header.head.arm7_entry = entry_point;
    header.head.arm7_offset = arm7_offset;

    header.head.title.copy_from_slice(b"HOMEBREW    ");
    header.head.unit_code = 3;

    let header_as_bytes = unsafe {
        core::slice::from_raw_parts(
            core::ptr::addr_of!(header) as *const u8,
            core::mem::size_of_val(&header),
        )
    };
    rom[..header_as_bytes.len()].copy_from_slice(header_as_bytes);

    Ok(rom)
}

fn construct_tmd(elf_file_path: PathBuf) -> Result<Vec<u8>, BuildError> {
    ///PLEASE DONT TOUCH THIS, ITS VITAL TO THE EXPLOITS FUNCTION
    const M_STATE_OVERWRITE: &[u8] = &[
        84, 72, 73, 83, 32, 73, 83, 0, 0, 0, 0, 0, 223, 0, 0, 0, 87, 72, 69, 82, 69, 32, 84, 72,
        69, 0, 0, 0, 0, 0, 0, 0, 77, 65, 71, 73, 67, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 72, 65, 80,
        80, 69, 78, 68, 83, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 128, 242,
        125, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 1, 0, 0, 0, 192, 14, 127, 3, 0, 0, 0, 0, 0, 0, 0, 0,
    ];
    const M_STATE_OFFSET: usize = 0x13250;
    const _MIN_EXPLOIT_LEN: usize = 0x13C01;
    const USED_EXPLOIT_LEN: usize = 81400;
    const MAGIC_START_POINT: usize = 0x37DF06C;
    const M_ENTRYPOINT_LOCATION: usize = 0x1329C;

    info!("SELECTED ELF: {:?}", &elf_file_path);
    let file =
        fs::read(elf_file_path).map_err(|e| Crate::TMD.err()(CompileError::ElfNotFound(e)))?;
    let parse = ElfBytes::<AnyEndian>::minimal_parse(&file[..])
        .map_err(|e| Crate::TMD.err()(CompileError::ElfParseError(e)))?;
    let entrypoint = parse.ehdr.e_entry;
    //let rodata = parse.section_header_by_name(".rodata").unwrap().unwrap();
    let mut empty_tmd = vec![0u8; USED_EXPLOIT_LEN];

    let Some(segments) = parse.segments() else {
        return Err(BuildError {
            compile_error: CompileError::ElfMissingSegments,
            crate_type: Crate::TMD,
        });
    };
    let entry_point = entrypoint - (MAGIC_START_POINT as u64);
    let entry_value = (entrypoint as u32) + 4;
    info!(
        "Elf entrypoint: {}, file offset: {:x}, address: {:x}",
        entrypoint, entry_point, entry_value
    );
    for segment in segments.iter().filter(|f| f.p_type == 1 && f.p_filesz != 0) {
        let file_offset_start = (segment.p_vaddr as i64) - (MAGIC_START_POINT as i64);
        let file_offset_end = file_offset_start + segment.p_filesz as i64;
        if file_offset_start.is_negative() {
            continue;
        }
        let data = parse
            .segment_data(&segment)
            .map_err(|e| Crate::TMD.err()(CompileError::ElfSegmentError(e)))?;
        let file_range = (file_offset_start as usize)..(file_offset_end as usize);
        debug!(
            "Processing segment '{:x?}': {} bytes, file start: 0x{:x?}, file end: 0x{:x?}",
            segment.p_flags, segment.p_filesz, file_offset_start, file_offset_end
        );
        empty_tmd[file_range].copy_from_slice(data);
    }
    empty_tmd[M_STATE_OFFSET..][..M_STATE_OVERWRITE.len()].copy_from_slice(M_STATE_OVERWRITE);
    let values = entry_value.to_le_bytes();
    empty_tmd[M_ENTRYPOINT_LOCATION..][..values.len()].copy_from_slice(&values);

    Ok(empty_tmd)
}
#[derive(Parser)]
struct CompilerArgs {
    tmd_file: Option<PathBuf>,
    export_tmd: Option<PathBuf>,
}
impl TryFrom<CompilerArgs> for FixedCompilerArgs {
    type Error = &'static str;

    fn try_from(value: CompilerArgs) -> Result<Self, Self::Error> {
        Ok(Self {
            tmd_file: value
                .tmd_file
                .or_else(get_file)
                .ok_or("No path specified")?,
            export_tmd: value.export_tmd,
        })
    }
}
struct FixedCompilerArgs {
    tmd_file: PathBuf,
    export_tmd: Option<PathBuf>,
}
impl FixedCompilerArgs {
    fn build(self) -> Result<(), BuildError> {
        let env_us = env::current_dir().expect("Failed to get current dir using ENV");
        let arm9_path = env_us.clone().join("astronaut/arm9");
        let arm7_path = env_us.clone().join("astronaut/arm7");

        let arm9_bootstrap_path = env_us.clone().join("astronaut/bootstrap");
        let arm7_bootstrap_path = env_us.clone().join("astronaut/bs_arm7");

        let arm9_installer_path = env_us.clone().join("crates/installer/arm9");
        let arm7_installer_path = env_us.clone().join("crates/installer/arm7");

        let arm9_elf = env_us
            .clone()
            .join("target-binary/thumbv5te-none-eabi/release/DeBoot_arm9");
        let arm7_elf = env_us
            .clone()
            .join("target-binary/thumbv4t-none-eabi/release/DeBoot_arm7");

        let arm9_elf_installer = env_us
            .clone()
            .join("target-installer/armv5te-none-eabi/release/arm9");
        let arm7_elf_installer = env_us
            .clone()
            .join("target-installer/armv4t-none-eabi/release/arm7");

        let arm9_bs_elf = env_us
            .clone()
            .join("target-bootstrap/armv5te-none-eabi/release/arm9_bootstrap");
        // arm7 no longer needs a bootstrap since it's binary is already in VRAM (2025-12-06)
        /*
        let arm7_bs_elf = env_us
            .clone()
            .join("target-bootstrap/armv4t-none-eabi/release/arm7_bootstrap");
        */

        let arm7_include_path = env_us.clone().join("crates/hinddoor/hd_arm9/src/arm7.bin");
        let bootstrap_include_path = env_us
            .clone()
            .join("crates/hinddoor/hd_arm9/src/bootstrap.bin");

        //let span = span!(Level::TRACE, "Compiling Bootstrap");
        //let _enter = span.enter();
        build::build_crate(arm9_bootstrap_path).map_err(|e| (e, Crate::Arm9BootStrap))?;
        debug!("Built arm9 bootstrap");
        //build::build_crate(arm7_bootstrap_path).map_err(|e| (e, Crate::Arm7BootStrap))?;
        //debug!("Built arm7 bootstrap");
        //build::compile_bootstrap(arm9_bs_elf, bootstrap_include_path)
        //    .map_err(Crate::BootStrap.err())?;
        debug!("Done compiling bootstraps!");
        //drop(_enter);
        //let span = span!(Level::TRACE, "Arm7 binary");
        //let _enter = span.enter();
        //we have to do this idiotic thing or cargo craps itself with config.toml
        info!("Compiling ARM7 binary... ");
        build::build_crate(arm7_path).map_err(|e| (e, Crate::Arm7))?;
        //debug!("Done building AMR7!");

        //drop(_enter);
        //let span = span!(Level::TRACE, "Arm7 binary injection");
        //let _enter = span.enter();
        info!("Injecting into ARM7...");
        //build::compile_arm7(arm7_elf, arm7_include_path).map_err(Crate::Arm7.err())?;
        //debug!("Done injecting AMR7!");
        //drop(_enter);
        //let span = span!(Level::TRACE, "Arm9 binary");
        //let _enter = span.enter();
        info!("Compiling ARM9 binary... ");
        build::build_crate(arm9_path).map_err(|e| (e, Crate::Arm9))?;
        debug!("Done building ARM9!");

        /*
        build::build_crate(arm9_installer_path).map_err(|e| (e, Crate::Arm9BootStrap))?;
        debug!("Built arm9 installer");
        build::build_crate(arm7_installer_path).map_err(|e| (e, Crate::Arm7BootStrap))?;
        debug!("Built arm7 installer");
        */
        //drop(_enter);
        //let span = span!(Level::TRACE, "Arm9 binary injection");
        //let _enter = span.enter();
        info!("Resolving MMC image... ");
        debug!("Done building ARM9!");
        //drop(_enter);
        //let span = span!(Level::TRACE, "tmd");
        //let _enter = span.enter();
        let mmc_image_path = std::fs::canonicalize(&self.tmd_file).map_err(|_| BuildError {
            compile_error: CompileError::TMD(TMDCompileError::TMDFileMissing(self.tmd_file)),
            crate_type: Crate::TMD,
        })?;
        debug!("resolved to {:?}", mmc_image_path);
        info!("Injecting TMD into MMC image...");
        let exploited_tmd = construct_tmd(arm9_elf)?;
        mmc::write_tmd_to_image(mmc_image_path, &exploited_tmd).map_err(Crate::TMD.err())?;

        if let Some(mut path) = self.export_tmd {
            if fs::write(&path, &exploited_tmd[520..]).is_err() {
                error!("path for TMD export not available");
            }
            /*
            match construct_installer_rom(arm9_elf_installer, arm7_elf_installer) {
                Ok(installer) => {path.add_extension("dsi");
                if fs::write(&path, &installer).is_err() {
                    error!("Path for Installer rom not available");
                }},
                Err(err) => {
                    error!("Failed to build installer {err:?}");
                }
            }
            */
        }

        Ok(())
    }
}

fn get_file() -> Option<PathBuf> {
    /*
    FileDialog::new()
        .set_title("Select TMD to modify...")
        .pick_file()
     */
    None
}
fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let args: FixedCompilerArgs = match CompilerArgs::parse()
        .try_into()
        .map_err(|e: &'static str| e.to_owned())
    {
        Ok(e) => e,
        Err(e) => {
            error!("Could not get MMC file {e:?}");
            exit(1)
        }
    };
    match args.build() {
        Ok(()) => info!("Done"),
        Err(e) => error!("Failed to build {}", e),
    }
}

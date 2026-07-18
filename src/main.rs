// SPDX-FileCopyrightText: 2026 Viktor Karlsson <viktor@koda.re>
// SPDX-License-Identifier: GPL-3.0-or-later

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
use log::{debug, error, info, warn};

use self::errors::{BuildError, CompileError, Crate, TMDCompileError};
mod build;
mod errors;
mod mmc;
mod testing;

fn _inject_elf(
    elf_file_path: &PathBuf,
    memory: &mut [u8],
    start_addr: usize,
) -> Result<(), CompileError> {
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
    Ok(())
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
    const MAGIC_AUX_START_POINT: usize = 0x06880000 - (0x13800 + 520);
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
    for segment in segments.iter().filter(|f| f.p_type == 1 && f.p_memsz != 0) {
        let file_offset_start =  if segment.p_vaddr >= 0x06880000 {
            (segment.p_vaddr as i64) - (MAGIC_AUX_START_POINT as i64)
        } else {
            (segment.p_vaddr as i64) - (MAGIC_START_POINT as i64)
        };
        let file_offset_end = file_offset_start + segment.p_memsz as i64;
        if file_offset_start.is_negative() {
            continue;
        }
        let data = parse
            .segment_data(&segment)
            .map_err(|e| Crate::TMD.err()(CompileError::ElfSegmentError(e)))?;
        let file_range = (file_offset_start as usize)..(file_offset_end as usize);
        let label = match segment.p_flags {
            4 => "Read",
            5 => "Execute",
            6 => "Read+Write",
            _ => "Other",
        };
        debug!(
            "Processing segment '{}': {} bytes, file start: 0x{:x?}, file end: 0x{:x?}",
            label, segment.p_memsz, file_offset_start, file_offset_end
        );
        if empty_tmd.len() < file_range.end {
            let missing_bytes = file_range.end - empty_tmd.len();
            empty_tmd.reserve(missing_bytes);
            for _ in 0..missing_bytes {
                empty_tmd.push(0);
            }
        }
        if segment.p_filesz == 0 {
            for byte in empty_tmd[file_range].iter_mut() {
                *byte = 0;
            }
        } else {
            empty_tmd[file_range].copy_from_slice(data);
        }
    }
    empty_tmd[M_STATE_OFFSET..][..M_STATE_OVERWRITE.len()].copy_from_slice(M_STATE_OVERWRITE);
    let values = entry_value.to_le_bytes();
    empty_tmd[M_ENTRYPOINT_LOCATION..][..values.len()].copy_from_slice(&values);
    Ok(empty_tmd)
}
#[derive(Parser)]
struct CompilerArgs {
    export_tmd: Option<PathBuf>,
    nand_image_file: Option<PathBuf>,
}
impl TryFrom<CompilerArgs> for FixedCompilerArgs {
    type Error = &'static str;

    fn try_from(value: CompilerArgs) -> Result<Self, Self::Error> {
        Ok(Self {
            nand_image_file: value.nand_image_file,
            export_tmd: value.export_tmd,
        })
    }
}
struct FixedCompilerArgs {
    nand_image_file: Option<PathBuf>,
    export_tmd: Option<PathBuf>,
}
impl FixedCompilerArgs {
    fn build(self) -> Result<(), BuildError> {
        let env_us = env::current_dir().expect("Failed to get current dir using ENV");
        let arm9_path = env_us.clone().join("astronaut/arm9");
        let arm7_path = env_us.clone().join("astronaut/arm7");

        let arm9_bootstrap_path = env_us.clone().join("astronaut/bootstrap");

        let arm9_elf = env_us
            .clone()
            .join("target-binary/thumbv5te-none-eabi/release/arm9");

        info!("Compiling bootstrap binary... ");
        build::build_crate(arm9_bootstrap_path).map_err(|e| (e, Crate::Arm9BootStrap))?;
        info!("Compiling ARM7 binary... ");
        build::build_crate(arm7_path).map_err(|e| (e, Crate::Arm7))?;
        info!("Compiling ARM9 binary... ");
        build::build_crate(arm9_path).map_err(|e| (e, Crate::Arm9))?;
        debug!("Done building ARM9!");
        info!("Creating final binary...");
        let exploited_tmd = construct_tmd(arm9_elf)?;
        debug!("Done building stuff!");

        let path = self.export_tmd.unwrap_or(env_us.join("astronaut.bin"));
        match fs::write(&path, &exploited_tmd[520..]) {
            Ok(()) => info!("Final binary written to {:?}", &path),
            Err(e) => error!(
                "Failed to write the final binary to {:?}, error: {}",
                &path, e
            ),
        }

        if let Some(mmc_image_path) = &self.nand_image_file {
            info!("Resolving NAND image... ");
            let mmc_image_path = std::fs::canonicalize(mmc_image_path).map_err(|_| BuildError {
                compile_error: CompileError::TMD(TMDCompileError::TMDFileMissing(
                    mmc_image_path.clone(),
                )),
                crate_type: Crate::TMD,
            })?;

            debug!("resolved to {:?}", &mmc_image_path);
            info!("Injecting TMD into NAND image...");

            warn!("Using this method of install is unsafe and done at your own risk! It does not prevent the official firmware from deleting itself!");

            mmc::write_tmd_to_image(mmc_image_path, &exploited_tmd).map_err(Crate::TMD.err())?;
        } else {
            info!("No NAND image provided, skipping TMD injection.");
        }

        Ok(())
    }
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

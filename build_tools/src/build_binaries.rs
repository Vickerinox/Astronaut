use std::{
    fmt::Display,
    io::{Error, Write},
    path::PathBuf,
    process::Stdio,
};

use thiserror::Error;
use tracing::{debug, error, info};

#[derive(Debug)]
pub enum Crate {
    Arm9,
    Arm7,
    Arm9BootStrap,
    Arm7BootStrap,
    BootStrap,
    TMD,
    Arm9Installer,
    Arm7Installer,
}
impl Crate {
    pub fn err(self) -> impl FnOnce(CompileError) -> BuildError {
        |e| BuildError {
            compile_error: e,
            crate_type: self,
        }
    }
}
#[derive(Error, Debug)]
pub struct BuildError {
    pub compile_error: CompileError,
    pub crate_type: Crate,
}
impl Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Could not build {:?} \n {}",
            self.crate_type, self.compile_error
        )
    }
}
impl From<(CargoError, Crate)> for BuildError {
    fn from(value: (CargoError, Crate)) -> Self {
        BuildError {
            compile_error: CompileError::Cargo(value.0),
            crate_type: value.1,
        }
    }
}
#[derive(Error, Debug)]
pub enum CompileError {
    #[error("elf not found {0}")]
    ElfNotFound(IoError),
    #[error("elf could not be parsed {0}")]
    ElfParseError(ParseError),
    #[error("elf segment could not be parsed {0}")]
    ElfSegmentError(ParseError),
    #[error("elf is missing segments ")]
    ElfMissingSegments,
    #[error("bin file create failure {0}")]
    BinCreationFailure(IoError),
    #[error("bin could not be written {0}")]
    BinWriteFailute(IoError),

    #[error("could not run cargo command {0}")]
    Cargo(CargoError),
}
use elf::ParseError;
use std::io::Error as IoError;

#[derive(Error, Debug)]
pub enum CargoError {
    #[error("failed to spawn child {0}")]
    SpawnChild(IoError),
    #[error("failed command {0}")]
    FailedCommand(IoError),
    #[error("cargo command failed")]
    FailedProcess,
}

pub fn build_crate(path: PathBuf) -> Result<(), CargoError> {
    let mut cwd = std::process::Command::new("cargo")
        .arg("build")
        .arg("-r")
        .current_dir(&path)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| CargoError::SpawnChild(e))?;
    info!(
        "Spawning cargo command cargo build -r in {}",
        path.to_str().expect("already checked")
    );
    info!(
        "libclang path is: {:?}",
        std::env::var("LIBCLANG_PATH") /*
                                       std::env::set_var(
                                           "LIBCLANG_PATH",
                                           "/nix/store/2hn01gz32n3axgmzrcclivngcgkcxqbm-clang-21.1.7-lib/lib"
                                       )
                                       */
    );
    if !cwd
        .wait()
        .map_err(|e| CargoError::FailedCommand(e))?
        .success()
    {
        error!(
            "failed to run `cargo build -r` in {}`",
            path.to_str().expect("already checked")
        );
        return Err(CargoError::FailedProcess);
    }
    Ok(())
}

pub fn compile_arm7(
    elf_file_path: PathBuf,
    include_file_path: PathBuf,
) -> Result<Vec<u8>, CompileError> {
    const MAGIC_ENTRYPOINT_ADDRESS: usize = 0x600000C;
    const HEADER_SIZE: usize = 12;
    //const BLANK_BRANCH_INSTRUCTION: u32 = 0xEA000000;

    let file = std::fs::read(elf_file_path).map_err(|e| CompileError::ElfNotFound(e))?;
    let parse = elf::ElfBytes::<elf::endian::AnyEndian>::minimal_parse(&file[..])
        .map_err(|e| CompileError::ElfParseError(e))?;
    let entrypoint = parse.ehdr.e_entry;

    let mut empty_bin = vec![0u8; HEADER_SIZE];

    let segments = parse.segments().ok_or(CompileError::ElfMissingSegments)?;
    let entry_point = entrypoint - (MAGIC_ENTRYPOINT_ADDRESS as u64); // | 1; // 1 = THUMB
    let entry_value = ((entry_point as u32) >> 2).wrapping_sub(1) & 0xFFFFFF;

    /*
        ldr     r3, .L4
        bx      r3
        .L4:
        .word   <entry_point>
    */
    empty_bin[..(HEADER_SIZE - 4)]
        .copy_from_slice(&[0x00, 0x30, 0x9f, 0xe5, 0x13, 0xff, 0x2f, 0xe1]);
    empty_bin[(HEADER_SIZE - 4)..HEADER_SIZE]
        .copy_from_slice(&(entry_point as u32 + MAGIC_ENTRYPOINT_ADDRESS as u32).to_le_bytes());

    debug!(
        "Entry address: {:x} Entry value: {:x} Entry offset: {:x}",
        entrypoint, entry_point, entry_value
    );
    for segment in segments.iter().filter(|f| f.p_type == 1 && f.p_filesz != 0) {
        let file_offset_start =
            (segment.p_vaddr as i64) - (MAGIC_ENTRYPOINT_ADDRESS as i64) + (HEADER_SIZE as i64);
        let file_offset_end = file_offset_start + segment.p_filesz as i64;
        if file_offset_start.is_negative() {
            continue;
        }
        let data = parse
            .segment_data(&segment)
            .map_err(|e| CompileError::ElfSegmentError(e))?;
        if empty_bin.len() < file_offset_end as usize {
            let extra_len = (file_offset_end as usize) - empty_bin.len();
            empty_bin.append(&mut vec![0; extra_len]);
        }
        let file_range = (file_offset_start as usize)..(file_offset_end as usize);
        debug!(
            "SEGMENT '{}' OCCUPIED {} BYTES, {:x?}, {:x?} {:x?}",
            segment.p_type, segment.p_filesz, segment.p_vaddr, file_offset_start, file_offset_end
        );
        empty_bin[file_range].copy_from_slice(data);
    }

    while empty_bin.len() % 4 != 0 {
        empty_bin.push(0u8);
    }
    info!("ARM Binary is {:x?} bytes", empty_bin.len());

    info!("MISSION COMPLETE");
    Ok(empty_bin)
}

pub fn compile_bootstrap(
    elf9_file_path: PathBuf,
    bootstrap_file_path: PathBuf,
) -> Result<Vec<u8>, CompileError> {
    const HEADER_SIZE: usize = 12;
    const BLANK_BRANCH_INSTRUCTION: u32 = 0xEA000000;

    let arm9_file = std::fs::read(elf9_file_path).map_err(|e| CompileError::ElfNotFound(e))?;
    let arm9_parse = elf::ElfBytes::<elf::endian::AnyEndian>::minimal_parse(&arm9_file[..])
        .map_err(|e| CompileError::ElfParseError(e))?;
    let arm9_entrypoint = arm9_parse.ehdr.e_entry;

    let mut empty_bin = vec![255u8; HEADER_SIZE];

    let arm9_segments = arm9_parse
        .segments()
        .ok_or(CompileError::ElfMissingSegments)?;
    let arm9_entry_point = arm9_entrypoint;
    let arm9_entry_value = ((arm9_entry_point as u32) >> 2).wrapping_add(1) & 0xFFFFFF;

    empty_bin[..4].copy_from_slice(&(BLANK_BRANCH_INSTRUCTION | arm9_entry_value).to_ne_bytes());

    println!(
        "Entry address: {:x} Entry value: {:x} Entry offset: {:x}",
        arm9_entrypoint, arm9_entry_point, arm9_entry_value
    );
    let mut arm7_start = 0;
    for segment in arm9_segments
        .iter()
        .filter(|f| f.p_type == 1 && f.p_filesz != 0)
    {
        let file_offset_start = (segment.p_vaddr as i64) + (HEADER_SIZE as i64);
        let file_offset_end = file_offset_start + segment.p_filesz as i64;
        if file_offset_start.is_negative() {
            continue;
        }
        arm7_start = arm7_start.max(file_offset_end - HEADER_SIZE as i64);
        let data = arm9_parse
            .segment_data(&segment)
            .map_err(|e| CompileError::ElfSegmentError(e))?;
        if empty_bin.len() < file_offset_end as usize {
            let extra_len = (file_offset_end as usize) - empty_bin.len();
            empty_bin.append(&mut vec![0; extra_len]);
        }
        let file_range = (file_offset_start as usize)..(file_offset_end as usize);
        println!(
            "SEGMENT OCCUPIED {} BYTES, {:x?} {:x?}",
            segment.p_filesz, file_offset_start, file_offset_end
        );
        empty_bin[file_range].copy_from_slice(data);
    }

    /*
    let arm7_file = std::fs::read(elf7_file_path).map_err(|e| CompileError::ElfNotFound(e))?;
    let arm7_parse = elf::ElfBytes::<elf::endian::AnyEndian>::minimal_parse(&arm7_file[..])
        .map_err(|e| CompileError::ElfParseError(e))?;
    let arm7_entrypoint = arm7_parse.ehdr.e_entry;

    let arm7_segments = arm7_parse
        .segments()
        .ok_or(CompileError::ElfMissingSegments)?;
    let arm7_entry_point = arm7_entrypoint + arm7_start as u64;
    let arm7_entry_value = ((arm7_entry_point as u32) >> 2) & 0xFFFFFF;

    empty_bin[4..8].copy_from_slice(&(BLANK_BRANCH_INSTRUCTION | arm7_entry_value).to_ne_bytes());

    debug!(
        "Entry address: {:x} Entry value: {:x} Entry offset: {:x}",
        arm7_entrypoint, arm7_entry_point, arm7_entry_value
    );
    for segment in arm7_segments
        .iter()
        .filter(|f| f.p_type == 1 && f.p_filesz != 0)
    {
        let file_offset_start = (segment.p_vaddr as i64) + (HEADER_SIZE as i64) + arm7_start;
        let file_offset_end = file_offset_start + segment.p_filesz as i64;
        if file_offset_start.is_negative() {
            continue;
        }
        let data = arm7_parse
            .segment_data(&segment)
            .map_err(|e| CompileError::ElfSegmentError(e))?;
        if empty_bin.len() < file_offset_end as usize {
            let extra_len = (file_offset_end as usize) - empty_bin.len();
            empty_bin.append(&mut vec![0; extra_len]);
        }
        let file_range = (file_offset_start as usize)..(file_offset_end as usize);
        debug!(
            "SEGMENT OCCUPIED {} BYTES, {:x?} {:x?}",
            segment.p_filesz, file_offset_start, file_offset_end
        );
        empty_bin[file_range].copy_from_slice(data);
    }
    */
    assert!(
        empty_bin.len() < 0x4000,
        "WHATT TEHTEHETHTEHHETTE {:x}",
        empty_bin.len()
    );

    info!("MISSION COMPLETE");
    Ok(empty_bin)
}

// SPDX-FileCopyrightText: 2026 Viktor Karlsson <viktor@koda.re>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::{fmt::Display, path::PathBuf, process::Stdio};

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
    info!("libclang path is: {:?}", std::env::var("LIBCLANG_PATH"));
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

pub fn compile_arm7(elf_file_path: PathBuf) -> Result<Vec<u8>, CompileError> {
    let file = std::fs::read(elf_file_path).map_err(|e| CompileError::ElfNotFound(e))?;
    let parse = elf::ElfBytes::<elf::endian::AnyEndian>::minimal_parse(&file[..])
        .map_err(|e| CompileError::ElfParseError(e))?;
    assert_eq!(parse.ehdr.e_entry, 0x600_0000);

    let mut empty_bin = Vec::new();

    let segments = parse.segments().ok_or(CompileError::ElfMissingSegments)?;

    for segment in segments.iter().filter(|f| f.p_type == 1 && f.p_filesz != 0) {
        let file_offset_start = segment.p_paddr as i64 + 4;
        let file_offset_end = file_offset_start + segment.p_filesz as i64;
        if file_offset_start.is_negative() {
            continue;
        }
        let data = parse
            .segment_data(&segment)
            .map_err(|e| CompileError::ElfSegmentError(e))?;
        if empty_bin.len() < file_offset_end as usize {
            empty_bin.resize(file_offset_end as usize, 0);
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

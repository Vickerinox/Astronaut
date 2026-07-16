// SPDX-FileCopyrightText: 2026 Viktor Karlsson <viktor@koda.re>
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::mmc::mbr::MBRError;
use elf::ParseError;
use fatfs::Error as FatFsError;
use std::borrow::Cow;
use std::fmt::Display;
use std::io::Error as IoError;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug)]
pub enum Crate {
    Arm9,
    Arm7,
    Arm9BootStrap,
    TMD,
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
    _BinCreationFailure(IoError),
    #[error("bin could not be written {0}")]
    _BinWriteFailute(IoError),
    #[error("TMD compiling error {0}")]
    TMD(TMDCompileError),
    #[error("could not run cargo command {0}")]
    Cargo(CargoError),
}
impl From<TMDCompileError> for CompileError {
    fn from(value: TMDCompileError) -> Self {
        CompileError::TMD(value)
    }
}
#[derive(Error, Debug)]
pub enum TMDCompileError {
    #[error("tmd file not found at {0:?}")]
    TMDFileMissing(PathBuf),
    #[error("missing TMP footer")]
    MissingFooter,
    #[error("could not find mmc {0}")]
    MMCNotFound(IoError),
    #[error("could not read mmc {0}")]
    MMCRead(IoError),
    #[error("could not read mbr {0:?}")]
    MBR(MBRError),
    #[error("fat fs creation failed {0:?}")]
    FileSystemCreation(FatFsError<IoError>),
    #[error("fat fs failed writing {0:?}")]
    IOWrite(IoError),
    #[error("fat fs {0:?}")]
    Fatfs(FatFsError<IoError>),
    #[error("HWINFO.DAT not found {0:?}, is filesystem corrupted? ")]
    HWINFONotFound(FatFsError<IoError>),
    #[error("fat fs file {path} not found {source:?}")]
    FileNotFound {
        source: FatFsError<IoError>,
        path: Cow<'static, str>,
    },
    #[error("TMD file verification failed")]
    TMDFileVerification,
}
impl<C: Into<Cow<'static, str>>> From<(FatFsError<IoError>, C)> for TMDCompileError {
    fn from(value: (FatFsError<IoError>, C)) -> Self {
        let (source, p) = value;
        Self::FileNotFound {
            source,
            path: p.into(),
        }
    }
}

#[derive(Error, Debug)]
pub enum CargoError {
    #[error("failed to spawn child {0}")]
    SpawnChild(IoError),
    #[error("failed command {0}")]
    FailedCommand(IoError),
    #[error("cargo command failed")]
    FailedProcess,
}

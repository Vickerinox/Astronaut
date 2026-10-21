// SPDX-FileCopyrightText: 2026 Viktor Karlsson <viktor@koda.re>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::{path::PathBuf, process::Stdio};

use log::{debug, error};

use crate::errors::{CargoError};

pub fn build_crate(path: PathBuf, is_release: bool) -> Result<(), CargoError> {
    let mut cwd = std::process::Command::new("cargo");
    cwd.arg("build")
        .arg("-r")
        .current_dir(&path)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    if is_release {
        cwd.env_remove("ASTRONAUT_DEBUG");
    } else {
        // add git commit hash to env
        let output = std::process::Command::new("git")
            .args(&["rev-parse", "HEAD"])
            .output()
            .expect("You must have git installed to your default path in order to compile a non-release version of astronaut");
        let git_hash = String::from_utf8(output.stdout).unwrap();
        cwd.env("ASTRONAUT_DEBUG", &git_hash);
    }

    let mut cwd = cwd.spawn().map_err(CargoError::SpawnChild)?;
    debug!(
        "Spawning cargo command cargo build -r in {}",
        path.to_str().expect("already checked")
    );
    if !cwd.wait().map_err(CargoError::FailedCommand)?.success() {
        error!(
            "failed to run `cargo build -r` in {}`",
            path.to_str().expect("already checked")
        );
        return Err(CargoError::FailedProcess);
    }
    Ok(())
}
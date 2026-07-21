// SPDX-FileCopyrightText: 2026 Viktor Karlsson <viktor@koda.re>
// SPDX-License-Identifier: GPL-3.0-or-later

use build_tools::{compress, generate_font};
use std::{io::Write, process::Command};

fn main() {
    // find the out directory to put included files
    let out_dir = std::env::var_os("OUT_DIR").unwrap();

    // include the font in the binary
    let dest_path = std::path::Path::new(&out_dir).join("font_compressed.bin");
    let compressed_font = {
        let bitmap =
            generate_font(include_bytes!("./src/resources/font.bmp")).expect("invalid font bmp");
        compress(&bitmap)
    };
    std::fs::write(dest_path, compressed_font).unwrap();

    // include the bootstrap binary in the arm9 binary
    let bootstrap_path = std::path::Path::new(&out_dir).join("bootstrap.bin");
    let bootstrap_bin = {
        let bootstrap_file_path = std::path::PathBuf::from(
            "../../target-bootstrap/armv5te-none-eabi/release/arm9_bootstrap",
        );
        build_tools::build_binaries::compile_bootstrap(bootstrap_file_path.clone())
            .unwrap_or(Vec::new())
    }; //.expect("Please Compile the bootstrap binary before the main binary");

    std::fs::write(bootstrap_path, bootstrap_bin).unwrap();

    // include the arm7 binary in the arm9 binary
    let arm7_path = std::path::Path::new(&out_dir).join("arm7.bin");
    let arm7_bin = {
        let elf7_file_path =
            std::path::PathBuf::from("../../target-subbinary/thumbv4t-none-eabi/release/arm7");
        build_tools::build_binaries::compile_arm7(elf7_file_path.clone()).unwrap_or(Vec::new())
    }; //.expect("Please compile the ARM7 binary before the main binary");

    std::fs::write(arm7_path, arm7_bin).unwrap();
}

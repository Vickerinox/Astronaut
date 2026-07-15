use build_tools::{compress, generate_font};
use std::{io::Write, process::Command};

fn main() {
    // add git commit hash to env
    let output = Command::new("git")
        .args(&["rev-parse", "HEAD"])
        .output()
        .unwrap();
    let git_hash = String::from_utf8(output.stdout).unwrap();
    println!("cargo:rustc-env=GIT_HASH={}", &git_hash[..8]);

    // add font to build
    let out_dir = std::env::var_os("OUT_DIR").unwrap();
    let dest_path = std::path::Path::new(&out_dir).join("font_compressed.bin");
    let bitmap =
        generate_font(include_bytes!("./src/resources/font.bmp")).expect("invalid font bmp");
    let compressed_font = compress(&bitmap);
    std::fs::write(&dest_path, &compressed_font).unwrap();

    let _arm7_path = std::path::PathBuf::from("../../astronaut/arm7");
    let _bootstrap_path = std::path::PathBuf::from("../../astronaut/bootstrap");

    let dest_path = std::path::Path::new(&out_dir).join("bootstrap.bin");
    let elf9_file_path =
        std::path::PathBuf::from("../../target-bootstrap/armv5te-none-eabi/release/arm9_bootstrap");
    let boostrap_bin =
        build_tools::build_binaries::compile_bootstrap(elf9_file_path.clone())
            .unwrap_or(Vec::new()); //.expect("Please Compile the bootstrap binary before the main binary");

    std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(&dest_path)
        .and_then(|i| {
            i.set_len(boostrap_bin.len() as _)?;
            Ok(i)
        })
        .and_then(|mut i| i.write_all(&boostrap_bin))
        .expect("Failed to write bootstrap binary");

    let dest_path = std::path::Path::new(&out_dir).join("arm7.bin");
    let elf7_file_path =
        std::path::PathBuf::from("../../target-subbinary/thumbv4t-none-eabi/release/DeBoot_arm7");
    let boostrap_bin =
        build_tools::build_binaries::compile_arm7(elf7_file_path.clone())
            .unwrap_or(Vec::new()); //.expect("Please compile the ARM7 binary before the main binary");

    std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(&dest_path)
        .and_then(|i| {
            i.set_len(boostrap_bin.len() as _)?;
            Ok(i)
        })
        .and_then(|mut i| i.write_all(&boostrap_bin))
        .expect("Failed to write bootstrap binary");
}

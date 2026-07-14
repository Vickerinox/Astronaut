use build_tools::{compress, generate_font};
use std::process::Command;

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
    let bitmap = generate_font(include_bytes!("./resources/font.bmp")).expect("invalid font bmp");
    let compressed_font = compress(&bitmap);
    std::fs::write(&dest_path, &compressed_font).unwrap();

    let arm7_path = std::path::PathBuf::from("../../astronaut/arm7");
    let arm9_bootstrap_path = std::path::PathBuf::from("../../astronaut/bootstrap");
    
    let dest_path = std::path::Path::new(&out_dir).join("bootstrap.bin");
    let elf9_file_path = std::path::PathBuf::from("../../target-bootstrap/armv5te-none-eabi/release/arm9_bootstrap");
    let a = build_tools::build_binaries::compile_bootstrap(elf9_file_path, dest_path).expect("Please Compile the bootstrap binary before the main binary");

    let dest_path = std::path::Path::new(&out_dir).join("arm7.bin");
    let elf7_file_path = std::path::PathBuf::from("../../target-subbinary/thumbv4t-none-eabi/release/DeBoot_arm7");
    let a = build_tools::build_binaries::compile_arm7(elf7_file_path, dest_path).expect("Please compile the ARM7 binary before the main binary");

}

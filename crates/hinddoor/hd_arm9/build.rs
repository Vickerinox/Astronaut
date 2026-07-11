use std::process::Command;
use build_tools::{compress, generate_font};

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
}

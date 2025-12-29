use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut builder = cc::Build::new();
    let builder = builder
        .file("../cfatfs/fatfs/source/ff.c")
        .file("../cfatfs/fatfs/source/ffunicode.c")
        .target("armv5te-none-eabi")
        .compile("fatfs");
    Ok(())
}
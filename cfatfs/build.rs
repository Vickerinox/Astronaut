use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut builder = cc::Build::new();
    let builder = builder
        .file("fatfs/source/ff.c")
        .file("fatfs/source/ffunicode.c");
        
    builder.compile("fatfs");

    let target = env::var("TARGET")?;

    let bindings = bindgen::Builder::default()
        .header("fatfs/source/ff.h")
        .clang_arg(format!("--target={}", target))
        .use_core()
        .ctypes_prefix("cty")
        .derive_copy(false)
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    Ok(())
}
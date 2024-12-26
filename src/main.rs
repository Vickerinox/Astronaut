use std::{
    env::{self, Args},
    fs,
    path::PathBuf,
};

use elf::{endian::AnyEndian, ElfBytes};
use rfd::FileDialog;
mod build;
mod mmc;

fn construct_tmd(elf_file_path: PathBuf, mmc_file_path: PathBuf) {

    let mut og_tmd = std::fs::read("./title.tmd").unwrap();

    ///PLEASE DONT TOUCH THIS, ITS VITAL TO THE EXPLOITS FUNCTION
    const M_STATE_OVERWRITE: &[u8] = &[
        84, 72, 73, 83, 32, 73, 83, 0, 0, 0, 0, 0, 223, 0, 0, 0, 87, 72, 69, 82, 69, 32, 84, 72,
        69, 0, 0, 0, 0, 0, 0, 0, 77, 65, 71, 73, 67, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 72, 65, 80,
        80, 69, 78, 68, 83, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 128, 242,
        125, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 1, 0, 0, 0, 192, 14, 127, 3, 0, 0, 0, 0, 0, 0, 0, 0,
    ];
    const M_STATE_OFFSET: usize = 0x13250;
    const MIN_EXPLOIT_LEN: usize = 0x13C01;
    const USED_EXPLOIT_LEN: usize = 81400;
    const MAGIC_START_POINT: usize = 0x37DF06C;
    const M_ENTRYPOINT_LOCATION: usize = 0x1329C;

    println!("SELECTED ELF: {:?}", &elf_file_path);
    println!("SELECTED MMC: {:?}", &mmc_file_path);
    let file = fs::read(elf_file_path).unwrap();
    let parse = ElfBytes::<AnyEndian>::minimal_parse(&file[..]).unwrap();
    let entrypoint = parse.ehdr.e_entry;
    //let rodata = parse.section_header_by_name(".rodata").unwrap().unwrap();
    let mut empty_tmd = vec![0u8; USED_EXPLOIT_LEN];
    empty_tmd[..og_tmd.len()].copy_from_slice(&og_tmd);

    let Some(segments) = parse.segments() else {
        return;
    };
    let entry_point = entrypoint - (MAGIC_START_POINT as u64);
    let entry_value = (entrypoint as u32) + 4;
    println!("{} {:x} {:x}", entrypoint, entry_point, entry_value);
    for segment in segments.iter().filter(|f| f.p_type == 1 && f.p_filesz != 0) {
        let file_offset_start = (segment.p_vaddr as i64) - (MAGIC_START_POINT as i64);
        let file_offset_end = file_offset_start + segment.p_filesz as i64;
        if file_offset_start.is_negative() {
            continue;
        }
        let data = parse.segment_data(&segment).unwrap();
        let file_range = (file_offset_start as usize)..(file_offset_end as usize);
        println!(
            "OCCUPIED {} BYTES, {:x?} {:x?}",
            segment.p_filesz, file_offset_start, file_offset_end
        );
        empty_tmd[file_range].copy_from_slice(data);
    }
    empty_tmd[M_STATE_OFFSET..][..M_STATE_OVERWRITE.len()].copy_from_slice(M_STATE_OVERWRITE);
    let values = entry_value.to_le_bytes();
    empty_tmd[M_ENTRYPOINT_LOCATION..][..values.len()].copy_from_slice(&values);
    
    mmc::write_tmd_to_image(mmc_file_path, &empty_tmd).unwrap();

    println!("MISSION COMPLETE");
}
fn get_arg(args: &mut Args) -> Option<PathBuf> {
    args.next()
        .map(|s| {
            PathBuf::try_from(s).ok().or_else(|| {
                FileDialog::new()
                    .set_title("Select TMD to modify...")
                    .pick_file()
            })
        })
        .flatten()
}
fn main() {
    let mut args = env::args().into_iter();
    let _ = args.next();
    let env_us = env::current_dir().unwrap();
    let arm9_path = env_us.clone().join("arm9_bin");
    let arm7_path = env_us.clone().join("arm7_bin");

    let arm9_bootstrap_path = env_us.clone().join("arm9_bootstrap");
    let arm7_bootstrap_path = env_us.clone().join("arm7_bootstrap");

    let arm9_elf = env_us
        .clone()
        .join("target/armv5te-none-eabi/release/DeBoot_arm9");
    let arm7_elf = env_us
        .clone()
        .join("target/armv4t-none-eabi/release/DeBoot_arm7");

    let arm9_bs_elf = env_us
        .clone()
        .join("bs-target/armv5te-none-eabi/release/arm9_bootstrap");
    let arm7_bs_elf = env_us
        .clone()
        .join("bs-target/armv4t-none-eabi/release/arm7_bootstrap");

    let arm7_include_path = env_us.clone().join("arm9_bin/src/arm7.bin");
    let bootstrap_include_path = env_us.clone().join("arm9_bin/src/bootstrap.bin");

    print!("Compiling bootstrap...");
    build::build_crate(arm9_bootstrap_path).unwrap();
    build::build_crate(arm7_bootstrap_path).unwrap();

    build::compile_bootstrap(arm9_bs_elf, arm7_bs_elf, bootstrap_include_path).unwrap();

    println!("Done!");
    //we have to do this idiotic thing or cargo craps itself with config.toml
    print!("Compiling ARM7 binary... ");
    build::build_crate(arm7_path).unwrap();
    println!("Success!");
    print!("Injecting into ARM9...");
    build::compile_arm7(arm7_elf, arm7_include_path).unwrap();
    println!("Success!");
    print!("Compiling ARM9 binary... ");
    build::build_crate(arm9_path).unwrap();
    println!("Success!");
    print!("Resolving MMC image... ");
    let mmc_image_path = get_arg(&mut args).unwrap();
    println!("resolved to {:?}", mmc_image_path);
    println!("Injecting TMD into MMC image...");
    construct_tmd(arm9_elf, mmc_image_path);
}

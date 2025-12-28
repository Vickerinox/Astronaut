use common::bootstrap::{ArgvStructutre, HeaderTWL};
use common::bootstrap::{ARGV_MAGIC, SYSTEM_ARGV};
use reboot_lib::fatfs;
use reboot_lib::fatfs::SeekFrom;

use crate::BOOTSTRAP_BINARY;

unsafe fn inject_argv(header: &HeaderTWL, file_path: &str) {
    //find argv location
    let ntr_arg_destination = (header.arm9_load + header.arm9_size + 7) & !3;
    let arg_destination = if header.is_dsi_mode() {
        let twl_arg_destination = (header.arm9i_load + header.arm9i_size + 7) & !3;
        ntr_arg_destination.max(twl_arg_destination)
    } else {
        ntr_arg_destination
    };

    //declare the final argv
    let argv = arg_destination as *mut u8;
    let mut argv_size: usize = 0;

    //insert rom path
    {
        for byte in file_path.as_bytes() {
            argv.add(argv_size).write_volatile(*byte);
            argv_size += 1;
        }
        argv.add(argv_size).write_volatile(0);
        argv_size += 1;
    }

    //"initialize" final structure
    let final_argv_structure = ArgvStructutre {
        magic: ARGV_MAGIC,
        command_line: argv,
        command_length: argv_size as i32,
        argc: 0,
        argv: core::ptr::null_mut(),
        dummy: 0,
        host: 0,
    };
    //Copy to it's final location
    SYSTEM_ARGV.write_volatile(final_argv_structure);
}

pub unsafe fn boot_app<R: fatfs::Read + fatfs::Seek>(
    mut r: R,
    file_path: &str,
) -> Result<(), R::Error> {
    crate::stop_mod_file();
    let (header, _bootloader) = (*BOOTLOADER_MEM).split_at_mut(0x1000);
    //bootstrap::READY_FLAG_0.write_volatile(0xFF);
    //bootstrap::READY_FLAG_1.write_volatile(0xFF);
    //bootstrap::READY_FLAG_2.write_volatile(0xFF);
    //bootstrap::READY_FLAG_3.write_volatile(0xFF);

    r.read_exact(header)?;
    let header = &mut *(header as *mut [u8] as *mut u8 as *mut HeaderTWL);

    r.seek(SeekFrom::Start(header.arm9_offset as u64))?;
    let arm9_ram =
        core::slice::from_raw_parts_mut(header.arm9_load as *mut u8, header.arm9_size as usize);
    r.read_exact(arm9_ram)?;

    r.seek(SeekFrom::Start(header.arm9i_offset as u64))
        .expect("Failed to seek to ARM9i Binary");
    let arm9_ram =
        core::slice::from_raw_parts_mut(header.arm9i_load as *mut u8, header.arm9i_size as usize);
    r.read_exact(arm9_ram)?;

    r.seek(SeekFrom::Start(header.arm7_offset as u64))?;
    let arm9_ram =
        core::slice::from_raw_parts_mut(header.arm7_load as *mut u8, header.arm7_size as usize);
    r.read_exact(arm9_ram)?;

    r.seek(SeekFrom::Start(header.arm7i_offset as u64))?;
    let arm9_ram =
        core::slice::from_raw_parts_mut(header.arm7i_load as *mut u8, header.arm7i_size as usize);
    r.read_exact(arm9_ram)?;

    if header.is_homebrew() {
        inject_argv(header, file_path);
    }
    inject_bootstrap();
    (common::bootstrap::ARM9_JUMP as *mut u32).write_volatile(header.arm9_entry);
    reboot_lib::flush_mmc();

    while VCOUNT_REG.read_volatile() != 192 {}
    while VCOUNT_REG.read_volatile() == 192 {}
    reboot_lib::arm9_send_arm7_jump(header.arm7_entry);
    reboot_lib::disable_all_interrupts();
    const VCOUNT_REG: *const u16 = 0x4000006 as *const u16;
    while VCOUNT_REG.read_volatile() != 192 {}
    while VCOUNT_REG.read_volatile() == 192 {}

    core::ptr::write_volatile(0x4000000 as *mut u32, 0b00000000_00000001_00000000_00000000);
    core::ptr::write_volatile(0x5000000 as *mut u16, 0b0100001000010000);
    core::ptr::write_volatile(0x5000400 as *mut u16, 0b0100001000010000);

    reboot_lib::flush_mmc();
    (*(&common::bootstrap::ARM9_EN as *const usize as *const unsafe extern "C" fn()))();
    loop {}
}
pub unsafe fn inject_bootstrap() {
    //inject bootstrap into VRAM BANK I
    core::ptr::write_volatile(0x04000249 as *mut u8, 0x80); //enable VRAM bank I
    let mut stor: u32 = 0;
    for (i, byte) in BOOTSTRAP_BINARY.iter().enumerate() {
        stor |= (*byte as u32) << 24;
        if i & 3 == 3 {
            (common::bootstrap::BOOTLOADER_MEM as *mut u32)
                .add(i >> 2)
                .write_volatile(stor);
            stor = 0;
        } else {
            stor >>= 8;
        }
    }
    //bootstrap.copy_from_slice(crate::BOOTSTRAP_BINARY);
}

const BOOTLOADER_MEM: *mut [u8] =
    unsafe { core::slice::from_raw_parts_mut(0x2FFE000 as *mut u8, 0x2000) };

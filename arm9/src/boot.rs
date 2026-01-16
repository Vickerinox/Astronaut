use core::fmt::Debug;

use common::bootstrap::{ArgvStructutre, HeaderTWL};
use common::bootstrap::{ARGV_MAGIC, SYSTEM_ARGV};

pub enum BootError {
    BadBinaryLocation(core::ops::Range<u32>),
    BadEntrypoint(u32),

    FileReadError,
    FileSeekError,

    BadRomType,
}
impl Debug for BootError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BadBinaryLocation(arg0) => write!(f, "BadBinaryLocation {arg0:#10X?}"),
            Self::BadEntrypoint(arg0) => write!(f, "BadEntrypoint {arg0:#10X}"),
            Self::FileReadError => write!(f, "FileReadErr"),
            Self::FileSeekError => write!(f, "FileSeekErr"),
            BootError::BadRomType => write!(f, "BadROM"),
        }
    }
}
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

unsafe fn read_all(
    mut buffer: &mut [u8],
    file: &mut fatfs_embedded::fatfs::File,
) -> Result<(), fatfs_embedded::fatfs::Error> {
    loop {
        let bytes = fatfs_embedded::read(file, buffer)?;
        let Some(remaining) = buffer.get_mut((bytes as usize)..) else {
            return Err(fatfs_embedded::fatfs::Error::InternalLogicError);
        };
        if remaining.is_empty() {
            return Ok(());
        };
        buffer = remaining;
    }
}
#[inline]
unsafe fn boot_unreturnable(
    r: &mut fatfs_embedded::fatfs::File,
    file_path: &str,
    header: &HeaderTWL,
) -> ! {
    crate::stop_mod_file();

    let arm9_ram =
        core::slice::from_raw_parts_mut(header.arm9_load as *mut u8, header.arm9_size as usize);
    fatfs_embedded::seek(r, header.arm9_offset).unwrap();
    read_all(arm9_ram, r).unwrap();

    let arm9_ram =
        core::slice::from_raw_parts_mut(header.arm7_load as *mut u8, header.arm7_size as usize);
    fatfs_embedded::seek(r, header.arm7_offset).unwrap();
    read_all(arm9_ram, r).unwrap();

    if header.is_dsi_mode() {
        let arm9_ram = core::slice::from_raw_parts_mut(
            header.arm9i_load as *mut u8,
            header.arm9i_size as usize,
        );

        fatfs_embedded::seek(r, header.arm9i_offset).unwrap();
        read_all(arm9_ram, r).unwrap();

        let arm9_ram = core::slice::from_raw_parts_mut(
            header.arm7i_load as *mut u8,
            header.arm7i_size as usize,
        );
        fatfs_embedded::seek(r, header.arm7i_offset).unwrap();
        read_all(arm9_ram, r).unwrap();
    }

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
pub unsafe fn boot_app(
    r: &mut fatfs_embedded::fatfs::File,
    file_path: &str,
) -> BootError {
    let (header, _bootloader) = (*BOOTLOADER_MEM).split_at_mut(core::mem::size_of::<HeaderTWL>());
    if read_all(header, r).is_err() {
        return BootError::FileReadError;
    }
    let header = &*(header as *mut [u8] as *mut u8 as *mut HeaderTWL);

    let arm9_range = (header.arm9_load)..(header.arm9_load + header.arm9_size);
    let arm7_range = (header.arm7_load)..(header.arm7_load + header.arm7_size);
    if (!(0x200_0000..0x2FE_0000).contains(&arm9_range.start))
        || (!(0x200_0000..0x2FE_0000).contains(&arm9_range.end))
    {
        return BootError::BadBinaryLocation(arm9_range);
    }
    if (!(0x200_0000..0x2FE_0000).contains(&arm7_range.start))
        || (!(0x200_0000..0x2FE_0000).contains(&arm7_range.end))
    {
        return BootError::BadBinaryLocation(arm7_range);
    }
    if !arm7_range.contains(&header.arm7_entry) {
        return BootError::BadEntrypoint(header.arm7_entry);
    }
    if !arm9_range.contains(&header.arm9_entry) {
        return BootError::BadEntrypoint(header.arm9_entry);
    }
    boot_unreturnable(r, file_path, header);
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

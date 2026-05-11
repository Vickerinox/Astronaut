use core::fmt::Debug;

use common::bootstrap::{BOOTINFO_MEM, HeaderTWL};

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

unsafe fn read_all(
    mut buffer: &mut [u8],
    file: &mut fatfs_embedded::fatfs::File,
) -> Result<(), fatfs_embedded::fatfs::Error> {
    while !buffer.is_empty() {
        let bytes = fatfs_embedded::read(file, buffer)?;
        let Some(remaining) = buffer.get_mut((bytes as usize)..) else {
            return Err(fatfs_embedded::fatfs::Error::InternalLogicError);
        };
        buffer = remaining;
    }
    Ok(())
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

        if header.twl_flags & (1<<1) > 0 {
            match reboot_lib::arm9_decrypt_modcrypt(0) {
                Ok(()) => (),
                Err(code) => {panic!("Failed to modcrypt, code: {code}");},
            }
        }
    }

    if header.is_homebrew() {
        common::argv::init(header, file_path);
    }
    if header.is_dsiware() {
        common::device_list::init(header, "", "", file_path);
    }
    inject_bootstrap();
    (common::bootstrap::ARM9_JUMP as *mut u32).write_volatile(header.arm9_entry);
    reboot_lib::flush_mmc();

    while VCOUNT_REG.read_volatile() != 192 {}
    while VCOUNT_REG.read_volatile() == 192 {}
    let _boot_func = reboot_lib::arm9_send_arm7_jump(header.arm7_entry).unwrap_err();
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
    let mem = BOOTINFO_MEM as *mut () as *mut u32;
    for i in 0..0x1000 {
        mem.add(i).write_volatile(0);
    }
    let header = &mut (*common::bootstrap::BOOTINFO_MEM).twl_header;
    let head_buf = core::slice::from_raw_parts_mut(header as *mut HeaderTWL as *mut () as *mut u8, size_of::<HeaderTWL>());
    if read_all(head_buf, r).is_err() {
        return BootError::FileReadError;
    }
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
use core::fmt::Debug;

use common::{
    blowfish::BFCTX,
    bootstrap::{HeaderTWL, BOOTINFO_MEM},
};
use reboot_lib::{VIDEO_HARDWARE, swi_crc16};

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
use crate::{APP_AREA_START, AppArea, BOOTSTRAP_BINARY, INTERRUPT_TABLE, set_background};

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
    bf: &mut BFCTX,
) -> ! {
    crate::stop_mod_file();
    (*BOOTINFO_MEM).ntr.header_again = (*BOOTINFO_MEM).twl_header.head.clone();
    if header.is_homebrew() {
        let path_bytes = file_path.as_bytes();
        let (trim, path_bytes) = if path_bytes.get(..4) == Some(b"sdmc") {
            path_bytes.get(2..).map_or((false, path_bytes), |i| (true, i))
        } else {
            (false, path_bytes)
        };
        
        let other = &mut (*(APP_AREA_START as *mut AppArea)).path_buffer;
        for (i, o) in path_bytes.iter().zip(other.iter_mut()) {
            *o = *i; 
        }
        if trim {
            other[0..2].copy_from_slice(b"sd");
        }
        let path = core::str::from_raw_parts(other as *const u8, path_bytes.len());
        
        common::argv::init(header,path);
        reboot_lib::nocash_write("> Inserted ARGV \n");
        
        
    }
    
    let arm9_ram = core::slice::from_raw_parts_mut(
        header.head.arm9_load as *mut u8,
        header.head.arm9_size as usize,
    );
    fatfs_embedded::seek(r, header.head.arm9_offset).unwrap();
    read_all(arm9_ram, r).unwrap();

    reboot_lib::nocash_write("> ARM9 binary loaded \n");

    let arm9_ram = core::slice::from_raw_parts_mut(
        header.head.arm7_load as *mut u8,
        header.head.arm7_size as usize,
    );
    fatfs_embedded::seek(r, header.head.arm7_offset).unwrap();
    read_all(arm9_ram, r).unwrap();

    reboot_lib::nocash_write("> ARM7 binary loaded \n");

    
    if header.is_dsi_mode() {
        let arm9_ram = core::slice::from_raw_parts_mut(
            header.arm9i_load as *mut u8,
            header.arm9i_size as usize,
        );

        fatfs_embedded::seek(r, header.arm9i_offset).unwrap();
        read_all(arm9_ram, r).unwrap();

        reboot_lib::nocash_write("> ARM9i binary loaded \n");

        let arm9_ram = core::slice::from_raw_parts_mut(
            header.arm7i_load as *mut u8,
            header.arm7i_size as usize,
        );
        fatfs_embedded::seek(r, header.arm7i_offset).unwrap();
        read_all(arm9_ram, r).unwrap();

        reboot_lib::nocash_write("> ARM7i binary loaded \n");

        
        if header.head.twl_flags & (1 << 1) > 0 {
            match reboot_lib::arm9_decrypt_modcrypt(0) {
                Ok(()) => (),
                Err(code) => {
                    panic!("Failed to modcrypt, code: {code}");
                }
            }
            reboot_lib::nocash_write("> Applied Modcrypt \n");
        }
        
    }
    

    
    if (0x4000..0x8000).contains(&header.head.arm9_offset) {
        let tmp = header.head.arm9_load as *mut u32;
        if tmp.read() != 0xE7FFDEFF || tmp.add(1).read() != 0xE7FFDEFF {
            let gamecode = header.head.tid;
            let mut arg = [gamecode, gamecode >> 1, gamecode << 1];
            bf.init2(&mut arg);
            bf.init2(&mut arg);
            bf.decrypt(&mut *tmp.add(1), &mut *tmp);
            arg[1] <<= 1;
            arg[2] >>= 1;
            bf.init2(&mut arg);
            bf.decrypt(&mut *tmp.add(1), &mut *tmp);

            for i in (2..0x200).step_by(2) {
                bf.decrypt(&mut *tmp.add(i + 1), &mut *tmp.add(i));
            }
            if tmp.read() == 0x72636E65 && tmp.add(1).read() == 0x6A624F79 {
                tmp.write(0xE7FFDEFF);
                tmp.add(1).write(0xE7FFDEFF);
            }
            reboot_lib::nocash_write("> Decrypted Secure Area \n");
        }
    }
    


    common::device_list::init(header, "sdmc:/pub.sav", "sdmc:/prv.sav", file_path);
    reboot_lib::nocash_write("> Inserted Device List \n");
    
    {
        common::config::init(header);
        let wifi_type = (*BOOTINFO_MEM).ntr.firmware_data[0xFF];
        (0x20005E0 as *mut u8).write_volatile(wifi_type);
        if wifi_type == 2 || wifi_type == 3 {
            (0x20005E4 as *mut u32).write_volatile(0x520000);
            (0x20005E8 as *mut u32).write_volatile(0x520000);
            (0x20005EC as *mut u32).write_volatile(0x020000);
        } else {
            (0x20005E4 as *mut u32).write_volatile(0x500400);
            (0x20005E8 as *mut u32).write_volatile(0x500000);
            (0x20005EC as *mut u32).write_volatile(0x02E000);
        }
        (0x20005E2 as *mut u16).write_volatile(swi_crc16(0xFFFF, 0x020005E4 as *const (), 0xC));
        reboot_lib::nocash_write("> Inserted TWL_CONFIG \n");
    }
    
    inject_bootstrap();
    (common::bootstrap::ARM9_JUMP as *mut u32).write_volatile(header.head.arm9_entry);
    reboot_lib::flush_mmc();

   
    while (&*(APP_AREA_START as *mut AppArea)).fader.current.read() != (&*(APP_AREA_START as *mut AppArea)).fader.target.read() {}
    reboot_lib::disable_all_interrupts();
    core::ptr::write_volatile(0x4000000 as *mut u32, 0b00000000_00000001_00000000_00000000);
    if (&*(APP_AREA_START as *mut AppArea)).fader.current.read() > 15  {
        set_background(0x7FFF);
        crate::set_bright(0);
    }

    const VCOUNT_REG: *const u16 = 0x4000006 as *const u16;
    while VCOUNT_REG.read_volatile() != 192 {}
    while VCOUNT_REG.read_volatile() == 192 {}
    reboot_lib::flush_mmc();
    reboot_lib::flush_mmc();
    let _boot_func = reboot_lib::arm9_send_arm7_jump(header.head.arm7_entry).unwrap_err();
    while VCOUNT_REG.read_volatile() != 192 {}
    while VCOUNT_REG.read_volatile() == 192 {}
    (*(&common::bootstrap::ARM9_EN as *const usize as *const unsafe extern "C" fn()))();
    loop {}
}

pub unsafe fn boot_app(
    r: &mut fatfs_embedded::fatfs::File,
    file_path: &str,
    blowfish: &mut BFCTX,
) -> BootError {
    reboot_lib::nocash_write("> booting ");
    reboot_lib::nocash_write(file_path);
    reboot_lib::nocash_write("\n");
    let mem = BOOTINFO_MEM as *mut () as *mut u32;
    for i in 0..0xE00 {
        mem.add(i).write_volatile(0);
    }
    let header = &mut (*common::bootstrap::BOOTINFO_MEM).twl_header;
    let head_buf = core::slice::from_raw_parts_mut(
        header as *mut HeaderTWL as *mut () as *mut u8,
        size_of::<HeaderTWL>(),
    );
    if read_all(head_buf, r).is_err() {
        return BootError::FileReadError;
    }
    reboot_lib::nocash_write("> Loaded Header");
    let arm9_range = (header.head.arm9_load)..(header.head.arm9_load + header.head.arm9_size);
    let arm7_range = (header.head.arm7_load)..(header.head.arm7_load + header.head.arm7_size);
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
    if !arm7_range.contains(&header.head.arm7_entry) {
        return BootError::BadEntrypoint(header.head.arm7_entry);
    }
    if !arm9_range.contains(&header.head.arm9_entry) {
        return BootError::BadEntrypoint(header.head.arm9_entry);
    }

    boot_unreturnable(r, file_path, header, blowfish);
}
pub unsafe fn inject_bootstrap() {
    //inject bootstrap into VRAM BANK I
    core::ptr::write_volatile(0x04000249 as *mut u8, 0x80); //enable VRAM bank I
    for (i, byte) in BOOTSTRAP_BINARY.iter().enumerate() {
        (common::bootstrap::BOOTLOADER_MEM as *mut u8)
            .add(i)
            .write_volatile(*byte);
    }
}

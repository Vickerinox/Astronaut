use core::fmt::Debug;

use alloc::string::{String, ToString};
use common::bootstrap::{BootInfoTWL, TWLHeader, BOOTINFO_MEM};
use fatfs_embedded::fatfs::FileOptions;
use reboot_lib::{swi_crc16, DisplayControl, VIDEO_HARDWARE};

pub enum BootError {
    BadBinaryLocation(core::ops::Range<u32>),
    BadEntrypoint(u32),

    FileReadError,
}
impl Debug for BootError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BadBinaryLocation(arg0) => write!(f, "BadBinaryLocation {arg0:#10X?}"),
            Self::BadEntrypoint(arg0) => write!(f, "BadEntrypoint {arg0:#10X}"),
            Self::FileReadError => write!(f, "FileReadErr"),
        }
    }
}
use crate::{gui::GlobalData, set_background, AppArea, APP_AREA_START, BOOTSTRAP_BINARY};

pub fn read_all(
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

pub fn setup_shared_mem(mem: &mut BootInfoTWL) {
    mem.ntr.header_again = mem.twl_header.head.clone();
    mem.ntr.header = mem.twl_header.head.clone();

    let reset = 0;
    let rom_offset = 0;
    let boot_type = if mem.twl_header.is_dsiware() { 1 } else { 3 };

    mem.ntr.bootcheck.tid_1 = mem.twl_header.head.tid;
    mem.ntr.bootcheck.tid_2 = mem.twl_header.head.tid;
    mem.ntr.bootcheck.header_crc = mem.twl_header.head.header_crc;
    mem.ntr.bootcheck.secure_crc = mem.twl_header.head.secure_area_crc;
    mem.ntr.bootcheck.bios_crc = 0x5835;

    mem.ntr.reset = reset;
    mem.ntr.rom_offset = rom_offset;
    mem.ntr.boot_method.boot_type = boot_type;

    //for DSi mode only technically
    mem.sysmenu_id.clone_from_slice(b"00000009\0");
    mem.init_code = b'P';

    const HWINFO_TEMPLATE: [u8; 24] = [
        0x26, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x41, 0x41, 0x41, 0x31, 0x32, 0x33,
        0x34, 0x35, 0x36, 0x37, 0x38, 0x00, 0x00, 0x00, 0x3C,
    ];
    let hw_info_data =
        fatfs_embedded::open(&mut "nand:/SYS/HWINFO_S.dat".to_string(), FileOptions::Read)
            .ok()
            .and_then(|mut file| {
                fatfs_embedded::seek(&mut file, 0x88).ok()?;
                let mut buffer = [0u8; 24];
                read_all(&mut buffer, &mut file).ok()?;
                Some(buffer)
            })
            .unwrap_or(HWINFO_TEMPLATE);
    mem.ntr.hardware_info = hw_info_data;
}

#[inline]
unsafe fn boot_unreturnable(
    r: &mut fatfs_embedded::fatfs::File,
    file_path: &str,
    header: &mut BootInfoTWL,
    app_data: &mut crate::gui::GlobalData,
) -> ! {
    crate::stop_mod_file();
    let boot_info = header;

    if app_data.config.options.wifi_firmware_upload {
        //Launcher and hiyaCFW can skip wifi firmware load since they do it themselves
        if ![0x00030017_484E4100, 0x00030004_49485900]
            .contains(&(boot_info.twl_header.title_id & !0xFF))
        {
            crate::load_wifi_firmware();
        }
    }

    {
        let mut prv_path = String::with_capacity(file_path.len());
        let mut pub_path = String::with_capacity(file_path.len());

        if file_path.get(4..12) == Some(":/title/") {
            prv_path.push_str(&file_path[..30]);
            pub_path.push_str(&file_path[..30]);
            prv_path.push_str("data/private.sav");
            pub_path.push_str("data/public.sav");
        } else {
            prv_path.push_str(&file_path[..file_path.len() - 3]);
            pub_path.push_str(&file_path[..file_path.len() - 3]);
            prv_path.push_str("prv");
            pub_path.push_str("pub");
        }

        common::device_list::init(boot_info, file_path, &pub_path, &prv_path);
    }

    core::ptr::write_volatile(&mut boot_info.other[0], 0);

    setup_shared_mem(boot_info);
    if boot_info.twl_header.is_homebrew() {
        let path_bytes = file_path.as_bytes();
        let (trim, path_bytes) = if path_bytes.get(..4) == Some(b"sdmc") {
            path_bytes
                .get(2..)
                .map_or((false, path_bytes), |i| (true, i))
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

        common::argv::init(&boot_info.twl_header, path);
        reboot_lib::nocash_write("> Inserted ARGV \n");
    }

    unsafe { reboot_lib::ALLOCATOR.invalidate() };

    let arm9_ram = core::slice::from_raw_parts_mut(
        boot_info.twl_header.head.arm9_load as *mut u8,
        boot_info.twl_header.head.arm9_size as usize,
    );
    fatfs_embedded::seek(r, boot_info.twl_header.head.arm9_offset).unwrap();
    read_all(arm9_ram, r).unwrap();

    reboot_lib::nocash_write("> ARM9 binary loaded \n");

    let arm9_ram = core::slice::from_raw_parts_mut(
        boot_info.twl_header.head.arm7_load as *mut u8,
        boot_info.twl_header.head.arm7_size as usize,
    );
    fatfs_embedded::seek(r, boot_info.twl_header.head.arm7_offset).unwrap();
    read_all(arm9_ram, r).unwrap();

    reboot_lib::nocash_write("> ARM7 binary loaded \n");

    if boot_info.twl_header.is_dsi_mode() {
        let arm9_ram = core::slice::from_raw_parts_mut(
            boot_info.twl_header.arm9i_load as *mut u8,
            boot_info.twl_header.arm9i_size as usize,
        );

        fatfs_embedded::seek(r, boot_info.twl_header.arm9i_offset).unwrap();
        read_all(arm9_ram, r).unwrap();

        reboot_lib::nocash_write("> ARM9i binary loaded \n");

        let arm9_ram = core::slice::from_raw_parts_mut(
            boot_info.twl_header.arm7i_load as *mut u8,
            boot_info.twl_header.arm7i_size as usize,
        );
        fatfs_embedded::seek(r, boot_info.twl_header.arm7i_offset).unwrap();
        read_all(arm9_ram, r).unwrap();

        reboot_lib::nocash_write("> ARM7i binary loaded \n");

        if boot_info.twl_header.head.twl_flags & (1 << 1) > 0 {
            match reboot_lib::arm9_decrypt_modcrypt(0) {
                Ok(()) => (),
                Err(code) => {
                    panic!("Failed to modcrypt, code: {code}");
                }
            }
            reboot_lib::nocash_write("> Applied Modcrypt \n");
        }
    }

    if (0x4000..0x8000).contains(&boot_info.twl_header.head.arm9_offset) {
        let bf = &mut app_data.blowfish;
        let tmp = boot_info.twl_header.head.arm9_load as *mut u32;
        if tmp.read() != 0xE7FFDEFF || tmp.add(1).read() != 0xE7FFDEFF {
            let gamecode = boot_info.twl_header.head.tid;
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

    if app_data.config.options.patch_flag {
        common::patching::look_for_launcher_patch(&boot_info.twl_header);
    }
    reboot_lib::nocash_write("> Inserted Device List \n");
    {
        common::config::init(boot_info);
        let wifi_type = boot_info.ntr.hardware_info[23];
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
    (common::bootstrap::ARM9_JUMP as *mut u32).write_volatile(boot_info.twl_header.head.arm9_entry);
    reboot_lib::flush_mmc();

    while (&*(APP_AREA_START as *mut AppArea)).fader.current.read()
        != (&*(APP_AREA_START as *mut AppArea)).fader.target.read()
    {}
    reboot_lib::disable_all_interrupts();
    core::ptr::write_volatile(0x4000000 as *mut u32, 0b00000000_00000001_00000000_00000000);
    if (&*(APP_AREA_START as *mut AppArea)).fader.current.read() > 15 {
        set_background(0x7FFF);
        VIDEO_HARDWARE
            .geometry_commands
            .pipeline_swap_buffers
            .write(0);
        VIDEO_HARDWARE.engine_a_ctrl.write(DisplayControl::empty());
        VIDEO_HARDWARE.disp_b_control.write(DisplayControl::empty());

        crate::set_bright(0);
    }
    reboot_lib::flush_mmc();
    reboot_lib::flush_mmc();
    let _boot_func =
        reboot_lib::arm9_send_arm7_jump(boot_info.twl_header.head.arm7_entry).unwrap_err();
    (*(&common::bootstrap::ARM9_EN as *const usize as *const unsafe extern "C" fn()))();
    loop {}
}

pub unsafe fn boot_app(
    r: &mut fatfs_embedded::fatfs::File,
    file_path: &str,
    app_data: &mut GlobalData,
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
        header as *mut TWLHeader as *mut () as *mut u8,
        size_of::<TWLHeader>(),
    );
    if read_all(head_buf, r).is_err() {
        return BootError::FileReadError;
    }
    reboot_lib::nocash_write("> Loaded Header");

    if header.is_dsi_mode() {
        if header.arm9i_size != 0 {
            let arm9_range = (header.arm9i_load)..(header.arm9i_load + header.arm9i_size);
            if (!(0x200_0000..0x2FC_0000).contains(&arm9_range.start))
                || (!(0x200_0000..0x2FC_0000).contains(&arm9_range.end))
            {
                return BootError::BadBinaryLocation(arm9_range);
            }
        }
        if header.arm7i_size != 0 {
            let arm7_range = (header.arm7i_load)..(header.arm7i_load + header.arm7i_size);
            if (!(0x200_0000..0x2FC_0000).contains(&arm7_range.start))
                || (!(0x200_0000..0x2FC_0000).contains(&arm7_range.end))
            {
                return BootError::BadBinaryLocation(arm7_range);
            }
        }
    }

    let arm9_range = (header.head.arm9_load)..(header.head.arm9_load + header.head.arm9_size);
    let arm7_range = (header.head.arm7_load)..(header.head.arm7_load + header.head.arm7_size);
    if (!(0x200_0000..0x2FC_0000).contains(&arm9_range.start))
        || (!(0x200_0000..0x2FC_0000).contains(&arm9_range.end))
    {
        return BootError::BadBinaryLocation(arm9_range);
    }
    if (!(0x200_0000..0x2FC_0000).contains(&arm7_range.start))
        || (!(0x200_0000..0x2FC_0000).contains(&arm7_range.end))
    {
        return BootError::BadBinaryLocation(arm7_range);
    }
    if !arm7_range.contains(&header.head.arm7_entry) {
        return BootError::BadEntrypoint(header.head.arm7_entry);
    }
    if !arm9_range.contains(&header.head.arm9_entry) {
        return BootError::BadEntrypoint(header.head.arm9_entry);
    }
    let header = &mut *(BOOTINFO_MEM);
    boot_unreturnable(r, file_path, header, app_data);
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

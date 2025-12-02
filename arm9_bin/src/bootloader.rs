use common::bootstrap;
use reboot_lib::fatfs;
use reboot_lib::fatfs::SeekFrom;
use reboot_lib::sound::SOUND_HARDWARE;

use crate::BOOTSTRAP_BINARY;

pub unsafe fn boot_app<R: fatfs::Read + fatfs::Seek>(mut r: R) -> Result<(), R::Error> {
    crate::stop_mod_file();
    let (header, _bootloader) = (*BOOTLOADER_MEM).split_at_mut(0x1000);
    bootstrap::READY_FLAG_0.write_volatile(0xFF);
    bootstrap::READY_FLAG_1.write_volatile(0xFF);
    bootstrap::READY_FLAG_2.write_volatile(0xFF);
    bootstrap::READY_FLAG_3.write_volatile(0xFF);

    r.read_exact(header)?;
    let header = &mut *(header as *mut [u8] as *mut u8 as *mut HeaderNDS);

    r.seek(SeekFrom::Start(header.arm9_offset as u64))
        .expect("Failed to seek to ARM9 Binary");
    let arm9_ram =
        core::slice::from_raw_parts_mut(header.arm9_load as *mut u8, header.arm9_size as usize);
    r.read_exact(arm9_ram).expect("Failed to read ARM9 Binary");

    r.seek(SeekFrom::Start(header.arm9i_offset as u64))
        .expect("Failed to seek to ARM9i Binary");
    let arm9_ram =
        core::slice::from_raw_parts_mut(header.arm9i_load as *mut u8, header.arm9i_size as usize);
    r.read_exact(arm9_ram).expect("Failed to read ARM9i Binary");


    r.seek(SeekFrom::Start(header.arm7_offset as u64))
        .expect("Failed to seek to ARM7 Binary");
    let arm9_ram =
        core::slice::from_raw_parts_mut(header.arm7_load as *mut u8, header.arm7_size as usize);
    r.read_exact(arm9_ram).expect("Failed to read ARM7 Binary");
    
    r.seek(SeekFrom::Start(header.arm7i_offset as u64))
        .expect("Failed to seek to ARM7i Binary");
    let arm9_ram =
        core::slice::from_raw_parts_mut(header.arm7i_load as *mut u8, header.arm7i_size as usize);
    r.read_exact(arm9_ram).expect("Failed to read ARM7i Binary");

    
    inject_bootstrap();
    (common::bootstrap::ARM9_JUMP as *mut u32).write_volatile(header.arm9_entry);
    reboot_lib::flush_mmc();
    const VCOUNT_REG: *const u16 = 0x4000006 as *const u16;
    reboot_lib::disable_all_interrupts();
    while VCOUNT_REG.read_volatile() != 192 {}
    while VCOUNT_REG.read_volatile() == 192 {}
    reboot_lib::arm9_send_arm7_jump(header.arm7_entry);
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
            (common::bootstrap::BOOTLOADER_MEM as *mut u32).add(i>>2).write_volatile(stor);
            stor = 0;
        } else {
            stor >>= 8;
        }
    }
    //bootstrap.copy_from_slice(crate::BOOTSTRAP_BINARY);
}

const BOOTLOADER_MEM: *mut [u8] =
    unsafe { core::slice::from_raw_parts_mut(0x2FFC000 as *mut u8, 0x4000) };

#[repr(C)]
pub struct HeaderNDS {
    pub title: [u8; 12],
    pub tid: u32,
    pub developer: u16,
    pub unit: u8,
    pub encryption_seed: u8,
    pub device_capacity: u8,
    _reserved: [u8; 7],
    pub revision: u16,
    pub rom_version: u8,
    pub flags: u8,

    pub arm9_offset: u32,
    pub arm9_entry: u32,
    pub arm9_load: u32,
    pub arm9_size: u32,

    pub arm7_offset: u32,
    pub arm7_entry: u32,
    pub arm7_load: u32,
    pub arm7_size: u32,

    fnt_offset: u32,
    fnt_len: u32,

    fat_offset: u32,
    fat_len: u32,

    arm9_overlay_offset: u32,
    arm9_overlay_len: u32,

    arm7_overlay_offset: u32,
    arm7_overlay_len: u32,

    card_cnt: u32,
    card_cnt_secure: u32,
    icon_offset: u32,
    secure_area_crc: u16,
    secure_area_timeout: u16,

    arm9_autoload: u32,
    arm7_autoload: u32,

    secure_disable: [u8; 8],

    ntr_rom_size: u32,
    header_size: u32,

    unknown: u32,
    _reserved2: [u32; 13],

    logo: [u8; 156],
    logo_crc: u16,

    header_crc: u16,

    debugger: [u8; 32],
    pub global_mbks: [u32; 5],
    pub arm9_mbks: [u32; 3],
    pub arm7_mbks: [u32; 3],
    pub mbk9: u32,

    region: u32,
    access_control: u32,
    pub arm7_scfg: u32,
    pub dsi_flags: u32,

    pub arm9i_offset: u32,
    _reservedi: u32,
    pub arm9i_load: u32,
    pub arm9i_size: u32,

    pub arm7i_offset: u32,
    _reservedi2: u32,
    pub arm7i_load: u32,
    pub arm7i_size: u32,

    digest_ntr_offset: u32,
    digest_ntr_len: u32,
    digest_twl_offset: u32,
    digest_twl_len: u32,
    sector_hashtable_offset: u32,
    sector_hashtable_len: u32,
    block_hashtable_offset: u32,
    block_hashtable_len: u32,
    sector_size: u32,
    block_sectorcount: u32,
    icon_banner_size: u32,
    unknown2: u32,
    total_rom_size: u32,
    unknown3: [u32; 3],
    modcrypt1_offset: u32,
    modcrypt1_len: u32,
    modcrypt2_offset: u32,
    modcrypt2_len: u32,
    title_id: [u8; 8],
    public_save_size: u32,
    private_save_size: u32,
    _reserved3: [u8; 176],
    unknown4: [u32; 4],

    arm9_sha1: [u32; 5],
    arm7_sha1: [u32; 5],
    digest_sha1: [u32; 5],
    banner_sha1: [u32; 5],
    arm9i_sha1: [u32; 5],
    arm7i_sha1: [u32; 5],
    _reserved4: [u8; 40],
    arm9_sha1_unsecure: [u32; 5],
    _reserved5: [u8; 2636],
    debug: [u8; 0x180],
    rsa_signature: [u8; 0x80],
}


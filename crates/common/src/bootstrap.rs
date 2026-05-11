use core::ptr::write_volatile as w;

use crate::device_list::DeviceList;

#[inline(always)]
pub unsafe fn boot_arm9() -> ! {
    //disable interrupts
    w(0x4000208 as *mut u32, 0);
    
    let header = &(*BOOTINFO_MEM).twl_header;
    let is_twl = (*BOOTINFO_MEM).twl_header.is_dsi_mode();
    

    //Setup global MBKS (at this point both the arm9 and arm7 should have setup local MBKS)
    if is_twl {
        let gmbks = &header.global_mbks;
        //NTR mbk
        w(0x4000247 as *mut u8, header.wram_cnt);
        //TWL mbk
        w(0x4004040 as *mut u32, gmbks[0]);
        w(0x4004044 as *mut u32, gmbks[1]);
        w(0x4004048 as *mut u32, gmbks[2]);
        w(0x400404C as *mut u32, gmbks[3]);
        w(0x4004050 as *mut u32, gmbks[4]);
    } else {
        w(0x4000247 as *mut u8, 3);
        w(0x4004040 as *mut u32, 0);
        w(0x4004044 as *mut u32, 0);
        w(0x4004048 as *mut u32, 0);
        w(0x400404C as *mut u32, 0);
        w(0x4004050 as *mut u32, 0);
    }
    
    //Setup local MBKS
    if is_twl {
        w(0x4004054 as *mut u32, header.arm9_mbks[0]);
        w(0x4004058 as *mut u32, header.arm9_mbks[1]);
        w(0x400405C as *mut u32, header.arm9_mbks[2]);
    } else {
        w(0x0 as *mut u32, header.arm9_mbks[0]);
        w(0x0 as *mut u32, header.arm9_mbks[1]);
        w(0x0 as *mut u32, header.arm9_mbks[2]);
    }
    //clear all interrupts
    w(0x4004040 as *mut u32, 0);
    w(0x4000210 as *mut u32, 0);
    (0x4000214 as *mut u32).write_volatile(!0);

    while VCOUNT_REG.read_volatile() != 192 {}


    //Sync to ARM7
    while VCOUNT_REG.read_volatile() == 192 {}

    //Jump to Entrypoint
    let entry = ARM9_JUMP;
    (*(entry as *mut unsafe extern "C" fn()))();
    loop {}
}
#[inline(always)]
pub unsafe fn boot_arm7() -> ! {
    //disable all interrupts
    (0x4000208 as *mut u32).write_volatile(0);
    (0x4000210 as *mut u32).write_volatile(0);
    (0x4000218 as *mut u32).write_volatile(0);
    let header = &(*BOOTINFO_MEM).twl_header;
    //setup MBKS
    if header.is_dsi_mode() {
        w(0x4004054 as *mut u32, header.arm7_mbks[0]);
        w(0x4004058 as *mut u32, header.arm7_mbks[1]);
        w(0x400405C as *mut u32, header.arm7_mbks[2]);
    } else {
        w(0x4004054 as *mut u32, 0);
        w(0x4004058 as *mut u32, 0);
        w(0x400405C as *mut u32, 0);
    }

    //clear all interrups
    (0x4000214 as *mut u32).write_volatile(!0);
    (0x400021C as *mut u32).write_volatile(!0);

    if header.arm7_device_list != 0 {
        core::arch::asm!("mov r11, r11");
        let dest = header.arm7_device_list as *mut u32;
        let src = core::ptr::addr_of!((*BOOTINFO_MEM).device_list_copy) as *const u32;
        for i in 0..0x100 {
            dest.add(i).write_volatile(*src.add(i));
        }
    }
    

    //Sync to ARM9
    while VCOUNT_REG.read_volatile() != 192 {}
    while VCOUNT_REG.read_volatile() == 192 {}

    //jump to entrypoint
    let entry = core::ptr::addr_of!(header.arm7_entry);
    (*(entry as *mut unsafe extern "C" fn()))();
    loop {}
}
pub const BOOTSTRAP_LOCATION: usize = 0x068A0000; //0x2FFD000;
pub const BOOTLOADER_MEM: *mut u8 = BOOTSTRAP_LOCATION as *mut u8;
pub const ARM9_EN: usize = BOOTSTRAP_LOCATION;
pub const ARM9_JUMP: usize = BOOTSTRAP_LOCATION + 4;
const VCOUNT_REG: *const u16 = 0x4000006 as *const u16;



#[repr(C)]
pub struct HeaderTWL {
    pub title: [u8; 12],
    pub tid: u32,
    pub maker_code: u16,
    pub unit_code: u8,
    pub encryption_seed: u8,
    pub device_capacity: u8,
    _0x15: [u8; 7],
    pub twl_flags: u8,
    pub ntr_flags: u8,
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

    pub fnt_offset: u32,
    pub fnt_len: u32,

    pub fat_offset: u32,
    pub fat_len: u32,

    pub arm9_overlay_offset: u32,
    pub arm9_overlay_len: u32,

    pub arm7_overlay_offset: u32,
    pub arm7_overlay_len: u32,

    pub card_cnt: u32,
    pub card_cnt_secure: u32,
    pub banner_offset: u32,
    pub secure_area_crc: u16,
    pub secure_area_timeout: u16,

    pub arm9_autoload_hook: u32,
    pub arm7_autoload_hook: u32,

    pub secure_disable: [u8; 8],

    pub ntr_rom_size: u32,
    pub header_size: u32,

    pub arm9_mod_params: u32,
    pub arm7_mod_params: u32,
    pub ntr_region_end: u16,
    pub twl_region_start: u16,
    pub nand_region_end: u16,
    pub nand_backup_start: u16,
    _0x98: [u8; 0x28],

    pub logo: [u8; 0x9C],

    pub logo_crc: u16,
    pub header_crc: u16,

    pub debug_rom_offset: u32,
    pub debug_rom_size: u32,
    pub debug_rom_load: u32, //doubles as arm9 entry
    pub debug_arm7_entry: u32,

    _0x170: [u8; 16],
    pub global_mbks: [u32; 5],
    pub arm9_mbks: [u32; 3],
    pub arm7_mbks: [u32; 3],

    pub mbk9: [u8; 3],
    pub wram_cnt: u8,

    pub region: u32,
    pub access_control: u32,
    pub arm7_scfg: u32,
    pub dsi_flags: u32,

    pub arm9i_offset: u32,
    _0x1c4: u32,
    pub arm9i_load: u32,
    pub arm9i_size: u32,

    pub arm7i_offset: u32,
    pub arm7_device_list: u32,
    pub arm7i_load: u32,
    pub arm7i_size: u32,

    pub digest_ntr_offset: u32,
    pub digest_ntr_len: u32,
    pub digest_twl_offset: u32,
    pub digest_twl_len: u32,
    pub sector_hashtable_offset: u32,
    pub sector_hashtable_len: u32,
    pub block_hashtable_offset: u32,
    pub block_hashtable_len: u32,
    pub sector_size: u32,
    pub block_sectorcount: u32,
    pub icon_banner_size: u32,
    pub shared_file_0_size: u8,
    pub shared_file_1_size: u8,
    pub eula_version: u8,
    pub twl_management_flags: u8,
    pub total_twl_rom_size: u32,

    pub shared_file_2_size: u8,
    pub shared_file_3_size: u8,
    pub shared_file_4_size: u8,
    pub shared_file_5_size: u8,

    pub arm9i_module_params: u32,
    pub arm7i_module_params: u32,

    pub modcrypt1_offset: u32,
    pub modcrypt1_len: u32,
    pub modcrypt2_offset: u32,
    pub modcrypt2_len: u32,
    pub title_id: u64,
    pub public_save_size: u32,
    pub private_save_size: u32,
    _0x240: [u8; 176],
    pub parental_c_ratings: [u8; 16],
    pub arm9_sha1: [u32; 5],
    pub arm7_sha1: [u32; 5],
    pub digest_sha1: [u32; 5],
    pub banner_sha1: [u32; 5],
    pub arm9i_sha1: [u32; 5],
    pub arm7i_sha1: [u32; 5],
    pub ntr_header_sha1: [u32; 5],
    pub ntr_fat_sha1: [u32; 5],
    pub arm9_sha1_unsecure: [u32; 5],
    _0x3b4: [u8; 0xA4C],
    pub debug_args: [u8; 0x100],
    _0xf00: [u8; 0x80],
    pub rsa_signature: [u8; 0x80],
}

//DKA bootstub struct
const BOOTSTUB_MAGIC: u64 = 0x62757473746F6F62; // "bootstub"
const BOOTSTUB_LOCATION: *mut BootStub = 0x2FF4000 as *mut BootStub;
#[repr(C)]
pub struct BootStub {
    pub magic: u64,
    pub arm9_entry: *const (),
    pub arm7_entry: *const (),
    pub loader_size: u32,
}



impl HeaderTWL {
    pub fn is_dsi_mode(&self) -> bool {
        self.unit_code & 2 > 0
    }
    pub fn is_dsiware(&self) -> bool {
        self.is_dsi_mode() && ((self.title_id << 32) & 0xFF) != 0
    }
    pub fn is_homebrew(&self) -> bool {
        self.maker_code == 0 || self.arm9_autoload_hook == 0 || self.arm7_load >= 0x03000000
    }
}


pub const BOOTINFO_MEM: *mut BootInfoTWL = 0x2FFC000 as *mut BootInfoTWL;

#[repr(C)]
pub struct BootInfoTWL {
    card_header: HeaderTWL,
    _0x1000: [u8; 0x3B0],
    pub device_list_copy: DeviceList,
    sysmenu_id: [u8; 9],
    init_code: u8,
    hotboot: u16,
    sdmmc_context: [u8; 0x44],
    title_list: [u8; 0x400],
    pub mountinfo: [u8; 0x3C0],
    pub boot_path: [u8; 0x40],
    pub twl_header: HeaderTWL,
    other: [u8; 0x680],
    _0x3680: [u8; 0x180],
    ntr: BootInfoNTR,
}
#[repr(C)]
pub struct BootInfoNTR {
    _0x0: [u8; 0x800]
}


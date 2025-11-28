use core::ptr::{read_volatile as r, write_volatile as w};

#[inline(always)]
pub unsafe fn boot_arm9() -> ! {
    (0x4000208 as *mut u32).write_volatile(0);

    READY_FLAG_1.write_volatile(0);
    while READY_FLAG_2.read_volatile() != 0 {}
    let mbks = core::ptr::addr_of!((*HEADER_MEM).global_mbks) as *const u32;
    for i in 0..8 {
        let value = r(mbks.add(i));
        w((0x4004040 as *mut u32).add(i), 0);
    }

    READY_FLAG_3.write_volatile(0);
    

    
    while READY_FLAG_0.read_volatile() != READY_VALUE {}

    (0x4000214 as *mut u32).write_volatile(!0);
    let entry = (*HEADER_MEM).arm9_entry;
    (*(entry as *mut unsafe extern "C" fn()))();
    loop {}
}
#[inline(always)]
pub unsafe fn boot_arm7() -> ! {
    (0x4000208 as *mut u32).write_volatile(0);
    //(0x4004060 as *mut u32).write_volatile(0xFFFF0F);
    READY_FLAG_2.write_volatile(0);
    while READY_FLAG_3.read_volatile() != 0 {}
    
    let mbks = core::ptr::addr_of!((*HEADER_MEM).arm7_mbks) as *const u32;
    for i in 0..4 {
        let value = r(mbks.add(i));
        w((0x4004054 as *mut u32).add(i), 0);
    }
    
    while VCOUNT_REG.read_volatile() != 192 {}
    READY_FLAG_0.write_volatile(READY_VALUE);
    (0x4000214 as *mut u32).write_volatile(!0);
    (0x400021C as *mut u32).write_volatile(!0);
    let entry = (*HEADER_MEM).arm7_entry;
    (*(entry as *mut unsafe extern "C" fn()))();
    loop {}
}
const HEADER_MEM: *const HeaderNDS = 0x2FFC000 as *const HeaderNDS;
pub const BOOTSTRAP_LOCATION: usize = 0x2FFD000;
pub const BOOTLOADER_MEM: *mut u8 = BOOTSTRAP_LOCATION as *mut u8;
pub const ARM9_EN: usize = BOOTSTRAP_LOCATION;
pub const ARM7_EN: usize = BOOTSTRAP_LOCATION + 4;
pub const READY_FLAG_0: *mut u8 = (0x2FFD008 + 0) as *mut u8;
pub const READY_FLAG_1: *mut u8 = (0x2FFD008 + 1) as *mut u8;
pub const READY_FLAG_2: *mut u8 = (0x2FFD008 + 2) as *mut u8;
pub const READY_FLAG_3: *mut u8 = (0x2FFD008 + 3) as *mut u8;
const READY_VALUE: u8 = 0;
const VCOUNT_REG: *const u16 = 0x4000006 as *const u16;

#[repr(C)]
struct HeaderNDS {
    title: [u8; 12],
    tid: u32,
    developer: u16,
    unit: u8,
    encryption_seed: u8,
    device_capacity: u8,
    _reserved: [u8; 7],
    revision: u16,
    rom_version: u8,
    flags: u8,

    arm9_offset: u32,
    arm9_entry: u32,
    arm9_load: u32,
    arm9_size: u32,

    arm7_offset: u32,
    arm7_entry: u32,
    arm7_load: u32,
    arm7_size: u32,

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
    global_mbks: [u32; 5],
    arm9_mbks: [u32; 3],
    arm7_mbks: [u32; 3],
    mbk9: u32,

    region: u32,
    access_control: u32,
    arm7_scfg: u32,
    dsi_flags: u32,

    arm9i_offset: u32,
    _reservedi: u32,
    arm9i_load: u32,
    arm9i_size: u32,

    arm7i_offset: u32,
    _reservedi2: u32,
    arm7i_load: u32,
    arm7i_size: u32,

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

//DKA argv struct
const ARGV_MAGIC: u32 = 0x5F617267; // "_arg"
const ARGV_LOCATION: *mut ArgV = 0x2FFFE70 as *mut ArgV;
#[repr(C)]
pub struct ArgV {
    pub magic: u32,
    pub cmdline: *mut core::ffi::c_char,
    pub cmdline_size: u32,
}
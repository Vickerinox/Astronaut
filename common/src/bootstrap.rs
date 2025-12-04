use core::ptr::{addr_of, read_volatile as r, write_volatile as w};

#[inline(always)]
pub unsafe fn boot_arm9() -> ! {
    (0x4000208 as *mut u32).write_volatile(0);
    (0x4000210 as *mut u32).write_volatile(0);
    let is_twl = (*HEADER_MEM).is_dsi_mode();
    if is_twl {
        w(0x4004054 as *mut u32, (*HEADER_MEM).arm9_mbks[0]);
        w(0x4004058 as *mut u32, (*HEADER_MEM).arm9_mbks[1]);
        w(0x400405C as *mut u32, (*HEADER_MEM).arm9_mbks[2]);
    } else {
        w(0x0 as *mut u32, (*HEADER_MEM).arm9_mbks[0]);
        w(0x0 as *mut u32, (*HEADER_MEM).arm9_mbks[1]);
        w(0x0 as *mut u32, (*HEADER_MEM).arm9_mbks[2]);
    }
    (0x4000214 as *mut u32).write_volatile(!0);
    while VCOUNT_REG.read_volatile() != 192 {}
    
    if is_twl {
        w(0x4000247 as *mut u8, (*HEADER_MEM).wram_cnt);
        w(0x4004040 as *mut u32, 0x0D090501);
        w(0x4004044 as *mut u32, (*HEADER_MEM).global_mbks[1]);
        w(0x4004048 as *mut u32, (*HEADER_MEM).global_mbks[2]);
        w(0x400404C as *mut u32, (*HEADER_MEM).global_mbks[3]);
        w(0x4004050 as *mut u32, (*HEADER_MEM).global_mbks[4]);
    } else {
        w(0x4000247 as *mut u8, 3);
        w(0x4004040 as *mut u32, 0);
        w(0x4004044 as *mut u32, 0);
        w(0x4004048 as *mut u32, 0);
        w(0x400404C as *mut u32, 0);
        w(0x4004050 as *mut u32, 0);
    }

    
    while VCOUNT_REG.read_volatile() == 192 {}

    let entry = ARM9_JUMP;
    (*(entry as *mut unsafe extern "C" fn()))();
    loop {}
}
#[inline(always)]
pub unsafe fn boot_arm7() -> ! {
    (0x4000208 as *mut u32).write_volatile(0);
    (0x4000210 as *mut u32).write_volatile(0);
    (0x4000218 as *mut u32).write_volatile(0);
    if (*HEADER_MEM).is_dsi_mode() {
        w(0x4004054 as *mut u32, (*HEADER_MEM).arm7_mbks[0]);
        w(0x4004058 as *mut u32, (*HEADER_MEM).arm7_mbks[1]);
        w(0x400405C as *mut u32, (*HEADER_MEM).arm7_mbks[2]);
    } else {
        w(0x0 as *mut u32, (*HEADER_MEM).arm7_mbks[0]);
        w(0x0 as *mut u32, (*HEADER_MEM).arm7_mbks[1]);
        w(0x0 as *mut u32, (*HEADER_MEM).arm7_mbks[2]);  
    }


    (0x4000214 as *mut u32).write_volatile(!0);
    (0x400021C as *mut u32).write_volatile(!0);
    while VCOUNT_REG.read_volatile() != 192 {}
    while VCOUNT_REG.read_volatile() == 192 {}

    let entry = core::ptr::addr_of!((*HEADER_MEM).arm7_entry);
    (*(entry as *mut unsafe extern "C" fn()))();
    loop {}
}
const HEADER_MEM: *const HeaderNDS = 0x2FFC000 as *const HeaderNDS;
pub const BOOTSTRAP_LOCATION: usize = 0x068A0000; //0x2FFD000;
pub const BOOTLOADER_MEM: *mut u8 = BOOTSTRAP_LOCATION as *mut u8;
pub const ARM9_EN: usize = BOOTSTRAP_LOCATION;
pub const ARM9_JUMP: usize = BOOTSTRAP_LOCATION + 4;
pub const READY_FLAG_0: *mut u8 = BOOTLOADER_MEM.wrapping_add(8);
pub const READY_FLAG_1: *mut u8 = BOOTLOADER_MEM.wrapping_add(9);
pub const READY_FLAG_2: *mut u8 = BOOTLOADER_MEM.wrapping_add(10);
pub const READY_FLAG_3: *mut u8 = BOOTLOADER_MEM.wrapping_add(11);
const READY_VALUE: u8 = 0;
const VCOUNT_REG: *const u16 = 0x4000006 as *const u16;

#[repr(C)]
pub struct HeaderNDS {
    pub title: [u8; 12],
    pub tid: u32,
    maker_code: u16,
    unit_code: u8,
    encryption_seed: u8,
    device_capacity: u8,
    _reserved: [u8; 7],
    revision: u16,
    rom_version: u8,
    flags: u8,

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
    
    pub mbk9: [u8; 3],
    pub wram_cnt: u8,

    region: u32,
    access_control: u32,
    arm7_scfg: u32,
    dsi_flags: u32,

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
    title_id: u64,
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

pub const ARGV_MAGIC: i32 = 0x5f617267;
pub const SYSTEM_ARGV: *mut ArgvStructutre = 0x02FFFE70 as _;
//DKA argv struct
#[repr(C)]
pub struct ArgvStructutre {
    pub magic: i32,
    pub command_line: *mut u8,
    pub command_length: i32,
    pub argc: i32,
    pub argv: *mut *mut u8,
    pub dummy: i32,
    pub host: u32,
}

impl HeaderNDS {
    pub fn is_dsi_mode(&self) -> bool {
        self.unit_code & 2 > 0
    }
    pub fn is_dsiware(&self) -> bool {
        self.is_dsi_mode() && ((self.title_id >> 32) & 0xFF) != 0
    }
    pub fn is_homebrew(&self) -> bool {
        self.maker_code == 0 || self.arm9_autoload == 0 || self.arm7_load >= 0x03000000 
    }
}

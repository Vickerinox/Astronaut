use core::ptr::write_volatile as w;

use crate::device_list::DeviceList;

#[cfg(not(target_arch = "arm"))]
pub unsafe fn boot_arm9() -> ! { loop {}}
#[cfg(not(target_arch = "arm"))]
pub unsafe fn boot_arm7() -> ! { loop {}}

#[cfg(target_arch = "arm")]
#[instruction_set(arm::a32)]
pub unsafe fn boot_arm9() -> ! {
    //disable interrupts
    w(0x4004040 as *mut u32, 0);
    w(0x4000208 as *mut u32, 0);
    w(0x4000210 as *mut u32, 0);
    //Setup local MBKS
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
    //clear all interrupts
    (0x4000214 as *mut u32).write_volatile(!0);

    while core::ptr::read_volatile(&(*BOOTINFO_MEM).other[0]) != 1 {}

    //Setup global MBKS (at this point both the arm9 and arm7 should have setup local MBKS)
    if is_twl {
        let gmbks = &(*HEADER_MEM).global_mbks;
        //NTR mbk
        w(0x4000247 as *mut u8, (*HEADER_MEM).wram_cnt);
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

    core::ptr::write_volatile(&mut (*BOOTINFO_MEM).other[0], 2);
    while core::ptr::read_volatile(&(*BOOTINFO_MEM).other[0]) != 3 {}

    //Sync to ARM9
    while VCOUNT_REG.read_volatile() != 192 {}
    let entry = core::ptr::addr_of!((*HEADER_MEM).head.arm9_entry);
    while VCOUNT_REG.read_volatile() == 192 {}
    //Jump to Entrypoint
    (*(entry as *mut unsafe extern "C" fn()))();
    loop {}
}
#[cfg(target_arch = "arm")]
pub unsafe fn boot_arm7() -> ! {
    //disable all interrupts
    (0x4000208 as *mut u32).write_volatile(0);
    (0x4000210 as *mut u32).write_volatile(0);
    (0x4000218 as *mut u32).write_volatile(0);

    //setup MBKS
    if (*HEADER_MEM).is_dsi_mode() {
        w(0x4004054 as *mut u32, (*HEADER_MEM).arm7_mbks[0]);
        w(0x4004058 as *mut u32, (*HEADER_MEM).arm7_mbks[1]);
        w(0x400405C as *mut u32, (*HEADER_MEM).arm7_mbks[2]);
    } else {
        w(0x4004054 as *mut u32, 0);
        w(0x4004058 as *mut u32, 0);
        w(0x400405C as *mut u32, 0);
    }

    //clear all interrups
    (0x4000214 as *mut u32).write_volatile(!0);
    (0x400021C as *mut u32).write_volatile(!0);

    //Sync to ARM9
    core::ptr::write_volatile(&mut (*BOOTINFO_MEM).other[0], 1);
    while core::ptr::read_volatile(&(*BOOTINFO_MEM).other[0]) != 2 {}

    let header = &(*BOOTINFO_MEM).twl_header;
    if header.is_dsi_mode() {
        let device_list_location = header.arm7_device_list;
        if device_list_location != 0 {
            let dev_list_src =
                core::ptr::addr_of_mut!((*BOOTINFO_MEM).device_list_copy) as *const u32;
            for i in 0..(core::mem::size_of::<DeviceList>() / core::mem::size_of::<u32>()) {
                (device_list_location as *mut u32)
                    .add(i)
                    .write(dev_list_src.add(i).read());
            }
        }
    }
    core::ptr::write_volatile(&mut (*BOOTINFO_MEM).other[0], 3);

    //Sync to ARM9
    while VCOUNT_REG.read_volatile() != 192 {}
    let entry = core::ptr::addr_of!((*HEADER_MEM).head.arm7_entry);
    while VCOUNT_REG.read_volatile() == 192 {}
    //jump to entrypoint
    (*(entry as *mut unsafe extern "C" fn()))();
    loop {}
}
const HEADER_MEM: *const HeaderTWL = 0x2FFE000 as *const HeaderTWL;

pub const BOOTSTRAP_LOCATION: usize = 0x068A0000; //0x2FFD000;
pub const BOOTLOADER_MEM: *mut u8 = BOOTSTRAP_LOCATION as *mut u8;
pub const ARM9_EN: usize = BOOTSTRAP_LOCATION;
pub const ARM9_JUMP: usize = BOOTSTRAP_LOCATION + 4;
const VCOUNT_REG: *const u16 = 0x4000006 as *const u16;

#[repr(C)]
#[derive(Debug, Clone)]
pub struct HeaderStart {
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

    // info for loading the arm9 binary
    pub arm9_offset: u32,
    pub arm9_entry: u32,
    pub arm9_load: u32,
    pub arm9_size: u32,

    // info for loading the arm7 binary
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
}

const_assert!(core::mem::size_of::<HeaderStart>() == 0x160);

#[repr(C)]
pub struct HeaderTWL {
    pub head: HeaderStart,
    pub debug_rom_offset: u32,
    pub debug_rom_size: u32,
    pub debug_rom_load: u32, //doubles as arm9 entry?
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
const_assert!(core::mem::size_of::<HeaderTWL>() == 0x1000);

impl HeaderTWL {
    pub fn new() -> Self {
        Self {
            head: HeaderStart {
                title: Default::default(),
                tid: Default::default(),
                maker_code: Default::default(),
                unit_code: Default::default(),
                encryption_seed: Default::default(),
                device_capacity: Default::default(),
                _0x15: Default::default(),
                twl_flags: Default::default(),
                ntr_flags: Default::default(),
                rom_version: Default::default(),
                flags: Default::default(),
                arm9_offset: Default::default(),
                arm9_entry: Default::default(),
                arm9_load: Default::default(),
                arm9_size: Default::default(),
                arm7_offset: Default::default(),
                arm7_entry: Default::default(),
                arm7_load: Default::default(),
                arm7_size: Default::default(),
                fnt_offset: Default::default(),
                fnt_len: Default::default(),
                fat_offset: Default::default(),
                fat_len: Default::default(),
                arm9_overlay_offset: Default::default(),
                arm9_overlay_len: Default::default(),
                arm7_overlay_offset: Default::default(),
                arm7_overlay_len: Default::default(),
                card_cnt: Default::default(),
                card_cnt_secure: Default::default(),
                banner_offset: Default::default(),
                secure_area_crc: Default::default(),
                secure_area_timeout: Default::default(),
                arm9_autoload_hook: Default::default(),
                arm7_autoload_hook: Default::default(),
                secure_disable: Default::default(),
                ntr_rom_size: Default::default(),
                header_size: Default::default(),
                arm9_mod_params: Default::default(),
                arm7_mod_params: Default::default(),
                ntr_region_end: Default::default(),
                twl_region_start: Default::default(),
                nand_region_end: Default::default(),
                nand_backup_start: Default::default(),
                _0x98: [0; _],
                logo: [0; _],
                logo_crc: Default::default(),
                header_crc: Default::default(),
            },
            debug_rom_offset: Default::default(),
            debug_rom_size: Default::default(),
            debug_rom_load: Default::default(),
            debug_arm7_entry: Default::default(),
            _0x170: Default::default(),
            global_mbks: Default::default(),
            arm9_mbks: Default::default(),
            arm7_mbks: Default::default(),
            mbk9: Default::default(),
            wram_cnt: Default::default(),
            region: Default::default(),
            access_control: Default::default(),
            arm7_scfg: Default::default(),
            dsi_flags: Default::default(),
            arm9i_offset: Default::default(),
            _0x1c4: Default::default(),
            arm9i_load: Default::default(),
            arm9i_size: Default::default(),
            arm7i_offset: Default::default(),
            arm7_device_list: Default::default(),
            arm7i_load: Default::default(),
            arm7i_size: Default::default(),
            digest_ntr_offset: Default::default(),
            digest_ntr_len: Default::default(),
            digest_twl_offset: Default::default(),
            digest_twl_len: Default::default(),
            sector_hashtable_offset: Default::default(),
            sector_hashtable_len: Default::default(),
            block_hashtable_offset: Default::default(),
            block_hashtable_len: Default::default(),
            sector_size: Default::default(),
            block_sectorcount: Default::default(),
            icon_banner_size: Default::default(),
            shared_file_0_size: Default::default(),
            shared_file_1_size: Default::default(),
            eula_version: Default::default(),
            twl_management_flags: Default::default(),
            total_twl_rom_size: Default::default(),
            shared_file_2_size: Default::default(),
            shared_file_3_size: Default::default(),
            shared_file_4_size: Default::default(),
            shared_file_5_size: Default::default(),
            arm9i_module_params: Default::default(),
            arm7i_module_params: Default::default(),
            modcrypt1_offset: Default::default(),
            modcrypt1_len: Default::default(),
            modcrypt2_offset: Default::default(),
            modcrypt2_len: Default::default(),
            title_id: Default::default(),
            public_save_size: Default::default(),
            private_save_size: Default::default(),
            _0x240: [0; _],
            parental_c_ratings: Default::default(),
            arm9_sha1: Default::default(),
            arm7_sha1: Default::default(),
            digest_sha1: Default::default(),
            banner_sha1: Default::default(),
            arm9i_sha1: Default::default(),
            arm7i_sha1: Default::default(),
            ntr_header_sha1: Default::default(),
            ntr_fat_sha1: Default::default(),
            arm9_sha1_unsecure: Default::default(),
            _0x3b4: [0; _],
            debug_args: [0; _],
            _0xf00: [0; _],
            rsa_signature: [0; _],
        }
    }
    pub fn is_dsi_mode(&self) -> bool {
        self.head.unit_code & 2 > 0
    }
    pub fn is_dsiware(&self) -> bool {
        self.is_dsi_mode() && ((self.title_id << 32) & 0xFF) != 0
    }
    pub fn is_homebrew(&self) -> bool {
        self.head.maker_code == 0
            || self.head.arm9_autoload_hook == 0
            || self.head.arm7_load >= 0x03000000
    }
}

pub const BOOTINFO_MEM: *mut BootInfoTWL = 0x2FFC000 as *mut BootInfoTWL;

#[repr(C)]
pub struct BootInfoTWL {
    pub card_header: HeaderTWL,
    
    _0x1400: [u8; 0x7B0],
    pub sysmenu_id: [u8; 9],
    pub init_code: u8,
    pub hotboot: u16,
    pub sdmmc_context: SDMMCContext,
    pub title_list: TitleList,
    pub mountinfo: [u8; 0x3C0],
    pub boot_path: [u8; 0x40],
    pub twl_header: HeaderTWL,
    
    pub other: [u8; 0x280],
    // UNOFFICIAL
    pub device_list_copy: DeviceList,
    _0x3680: [u8; 0x180],
    pub ntr: BootInfoNTR,
}
const_assert!(core::mem::size_of::<BootInfoTWL>() == 0x4000);

#[repr(C)]
pub struct SDMMCContext {
    pub cid: [u8; 16],
    pub csd: [u8; 16],
    pub ocr: [u8; 4],
    pub scr: [u8; 8],
    pub rca: u16,
    pub slot: u16,
    pub hcs: u16,
    pub unknown0: u16,
    pub unknown1: u32,
    pub csr: u32,
    pub clock_ctl: u16,
    pub option: u16,
    pub unknown2: u16,
    pub device: u16,
}
const_assert!(core::mem::size_of::<SDMMCContext>() == 0x44);

#[repr(C)]
pub struct TitleList {
    length: u8,
    padding: [u8; 15],
    pub_flags: [u8; 16],
    prv_flags: [u8; 16],
    jmp_flags: [u8; 16],
    mkr_flags: [u8; 16],
    title_ids: [u64; 0x76],
}
const_assert!(core::mem::size_of::<TitleList>() == 0x400);

#[repr(C)]
pub struct BootCheckInfo {
    pub tid_1: u32,
    pub tid_2: u32,
    pub header_crc: u16,
    pub secure_crc: u16,
    pub missing_crc: u16,
    pub secure_bad: u16,
    pub bios_crc: u16,
    pub secure_disable: u16,
    pub sio_flag: u16,
    pub rtc_status: u8,
    pub random: u8,
    _0x18: [u8; 8],
}

#[repr(C)]
pub struct BootMethod {
    pub boot_type: u16,
    pub dlplay_info: [u8; 0x3C],
    _0x3e: u16,
}
#[repr(C)]
pub struct FirmwareData {

}
#[repr(C)]
pub struct BootInfoNTR {
    //_0x0: [u8; 0x800],
    pub gap: [u8; 0x280],
    pub header: HeaderStart,
    pub download: [u8; 0x20],
    pub bootcheck: BootCheckInfo,
    pub reset: u32,
    _0x0424: [u8; 8],
    pub rom_offset: u32,
    pub slot_2_info: [u8; 0xC],
    pub vblank_counter: u32,
    pub boot_method: BootMethod,
    pub firmware_data: [u8; 0x74],
    pub mac_address: [u8; 6],
    pub wifi_channels: [u8; 2],
    _0x4fc: [u8; 4],
    _0x500: [u8; 0x68],
    pub supported_languages: u32,
    _0x56c: [u8; 4],
    pub console_region: u8,
    pub serial_number: [u8; 11],
    pub unknown: [u8; 4],
    pub arm9_exceptions: [u8; 0x1C],
    pub arm9_excep_vector: u32,
    _0x5a0: [u8; 0x48],
    pub rtc: [u8; 8],
    pub sysconf: [u8; 6],
    pub arm9print: u8,
    pub arm7print: u8,
    pub arm9err: u8,
    pub arm7err: u8,
    _0x5fa: [u8; 6],

    pub header_again: HeaderStart,
    pub debugger_again: [u8; 0x20],
    pub arm9ipc: u32,
    pub arm7ipc: u32,
    pub arm9ipch: u32,
    pub arm7ipch: u32,
    pub last_mic: u32,
    pub last_mic_data: u16,
    pub wifi_call: u16,
    pub wifi_rssi: u16,
    pub slot2_info: u8,
    pub slot2_in: u8,
    pub arm7arg: u32,
    pub arm9thread: u32,
    pub arm7thread: u32,
    pub buttons: u16,
    pub touch: [u8; 4],
    pub autoload: u16,
    pub arm9lock: [u8; 8],
    pub arm7lock: [u8; 8],
    pub vramclock: [u8; 8],
    pub vramdlock: [u8; 8],
    pub wram0lock: [u8; 8],
    pub wram1lock: [u8; 8],
    pub slot1lock: [u8; 8],
    pub slot2lock: [u8; 8],
    pub initlock: [u8; 8],
    pub arm9memc: u16,
    pub arm7memc: u16,
    pub memcmd: u16,
    pub wat: u16,
}
const_assert!(core::mem::size_of::<BootInfoNTR>() == 0x800);

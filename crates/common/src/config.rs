use crate::bootstrap::BootInfoTWL;

#[repr(C)]
pub struct Config {
    //flags
    pub config_flags: u32,
    _0x4: u8,
    pub country: u8,
    pub language: u8,
    pub rtc_year: u8,
    pub rtc_offset: [u8; 8],
    pub eula: [u8; 8],
    _0x18: [u8; 2],
    pub alarm_hour: u8,
    pub alarm_minute: u8,
    _0x1c: [u8; 2],
    pub alarm_enable: u8,
    _0x1f: [u8; 2],
    pub system_menu_used_slots: u8,
    pub system_menu_free_slots: u8,
    _0x23: u8,
    pub unknown: u8,
    _0x25: [u8; 3],
    pub last_selected_tid: u64,
    //touch calibration
    pub tc_x1_adc: u16,
    pub tc_y1_adc: u16,
    pub tc_x1_pixrl: u8,
    pub tc_y1_pixrl: u8,
    pub tc_x2_adc: u16,
    pub tc_y2_adc: u16,
    pub tc_x2_pixrl: u8,
    pub tc_y2_pixrl: u8,
    pub unknown2: u32,
    _0x40: [u8; 4],
    //profile
    pub favorite_color: u8,
    _0x45: u8,
    pub birth_month: u8,
    pub birth_day: u8,
    pub nickname: [u8; 0x16],
    pub message: [u8; 0x36],
    //parental controls
    pub pc_flags: u8,
    _0x95: [u8; 6],
    pub pc_region: u8,
    pub pc_age: u8,
    pub pc_secret_question: u8,
    pub pc_unknown: u8,
    _0x9f: [u8; 2],
    pub pc_pin: [u8; 5],
    pub pc_secret_answer: [u16; 65],
}

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
enum Region {
    Japan = 0,
    America = 1,
    Europe = 2,
    Australia = 3,
    China = 4,
    Korea = 5,
}
/*
enum UserLanguage {
    Japanese = 0,
    English = 1,
    French = 2,
    German = 3,
    Italian = 4,
    Spanish = 5,
    Chinese = 6,
    Korean = 7,
}
*/
impl Region {
    pub fn from_gamecode(tid: u32) -> Region {
        let code = (tid >> 24) as u8;
        match code {
            b'J' => Region::Japan,
            b'E' | b'T' => Region::America,
            b'U' => Region::Australia,
            b'C' => Region::China,
            b'K' => Region::Korea,
            _ => Region::Europe,
        }
    }
    pub fn lang_bitmask(&self) -> u32 {
        match self {
            Region::Japan => 0x1,
            Region::America => 0x26,
            Region::Europe => 0x3E,
            Region::Australia => 0x2,
            Region::China => 0x40,
            Region::Korea => 0x80,
        }
    }
}
pub unsafe fn init(header: &BootInfoTWL) {
    let gamecode = Region::from_gamecode(header.twl_header.head.tid);
    let common = &header.ntr.firmware_data.bytes;
    let config = &mut *(0x2000400 as *mut Config);
    config.config_flags = 0x0100000F;
    config.country = 0x4E;
    config.language = 1;

    config.eula[0] = 1;

    config.system_menu_free_slots = 9;
    config.system_menu_used_slots = 30;
    config.unknown = 3;
    config.unknown2 = 0x0201209C;
    config.birth_month = common[3];
    config.favorite_color = common[2];
    config.birth_day = common[4];
    config.alarm_hour = common[0x52];
    config.alarm_minute = common[0x53];
    config.alarm_enable = common[0x56];
    config.rtc_year = common[0x66];
    config.nickname.copy_from_slice(&common[0x6..0x6 + 0x16]);
    config.message.copy_from_slice(&common[0x1C..0x1C + 0x36]);
    config.rtc_offset.copy_from_slice(&common[0x68..0x70]);

    (0x2FFFD68 as *mut u32).write_volatile(gamecode.lang_bitmask());
    (0x2FFFD6C as *mut u32).write_volatile(0);
    (0x2FFFD70 as *mut u8).write_volatile(gamecode as u8);
}

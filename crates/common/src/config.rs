// SPDX-FileCopyrightText: 2026 Viktor Karlsson <viktor@koda.re>
// SPDX-License-Identifier: MIT

use crate::bootstrap::BootInfoTWL;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct WifiConfig {
    pub version: u16,
    pub crc: u16,
    pub data_segment: u32,
    pub base_address: u32,
    pub length: u32,
    unknown: [u32; 0x10 / 4],
}
unsafe impl bytemuck::NoUninit for TWLSettings {}
unsafe impl bytemuck::Zeroable for TWLSettings {}
unsafe impl bytemuck::AnyBitPattern for TWLSettings {}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TWLSettings {
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
pub unsafe fn init(header: &mut BootInfoTWL) {
    let region = Region::from_gamecode(header.twl_header.head.tid);

    (0x2FFFD68 as *mut u32).write_volatile(region.lang_bitmask());
    (0x2FFFD6C as *mut u32).write_volatile(0);
    (0x2FFFD70 as *mut u8).write_volatile(region as u8);
    (0x2FFFDFC as *mut u32).write_volatile(0x200_0400);
}

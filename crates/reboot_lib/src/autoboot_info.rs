use alloc::string::String;

use crate::MemoryWrapper;


#[repr(C)]
pub struct UnlaunchAutobootParams {
    signature: [u8; 12],
    crc_len: u16,
    crc: u16,
    flags: u32,
    upper_bg_color: u16,
    lower_bg_color: u16,
    _0x18: [u8; 0x20],
    file_path: [u16; 0x104],
    _0x240: [u8; 0x1c0],
}
const_assert!(core::mem::size_of::<UnlaunchAutobootParams>() == 0x400);

impl UnlaunchAutobootParams {
    pub fn is_valid(&self) -> bool {
        if &self.signature != b"AutoLoadInfo" {
            return false;
        }
        if self.crc_len != 0x3F0 {
            return false;
        }
        let crc = unsafe {
            crate::swi_crc16(0xFFFF,(&self.flags) as *const u32 as *const (), self.crc_len as usize)
        };
        crc == self.crc
    }
    pub fn parse_path(&self) -> String {
        String::from_utf16(&self.file_path).unwrap_or_default()
    }
}
pub const UNLAUNCH_AUTOBOOT_PARAM: MemoryWrapper<UnlaunchAutobootParams> = MemoryWrapper(0x200_0800 as *mut _);

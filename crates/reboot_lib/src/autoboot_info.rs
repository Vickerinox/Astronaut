use alloc::string::String;

use crate::MemoryWrapper;


#[repr(C)]
pub struct UnlaunchAutobootParams {
    signature: [u8; 12],
    crc_len: u16,
    flags: u32,
    upper_bg_color: u16,
    lower_bg_color: u16,
    _0x18: [u8; 0x20],
    file_path: [u16; 0x104],
    _0x240: [u8; 0x1c0],
}

impl UnlaunchAutobootParams {
    pub fn is_valid(&self) -> bool {
        &self.signature == b"AutoLoadInfo"
    }
    pub fn parse_path(&self) -> String {
        String::from_utf16(&self.file_path).unwrap_or_default()
    }
}
pub const UNLAUNCH_AUTOBOOT_PARAM: MemoryWrapper<UnlaunchAutobootParams> = MemoryWrapper(0x200_0800 as *mut _);

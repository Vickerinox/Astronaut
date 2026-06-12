use alloc::string::String;

use crate::MemoryWrapper;

#[repr(C)]
pub struct BootMethods {
    pub optional: core::mem::MaybeUninit<OptBoot>,
    pub official: core::mem::MaybeUninit<OfficialBoot>,
    _0x400: [u8; 0x400],
    pub unlaunch: UnlaunchBoot,
}

#[repr(C)]
pub struct OptBoot {
    title_id: u64,
    _0x8: u8,
    flags: u8,
    old_dev: u16,
    unknown: u16,
    _0xe: u16,
    crc: u16,
    unknown2: u16,
    unknown_buffer: [u8; 0x2EC],
}
const_assert!(core::mem::size_of::<OptBoot>() == 0x300);

#[repr(C)]
pub struct OfficialBoot {
    gamecode: u32,
    unknown: u8,
    crc_length: u8,
    crc: u16,
    old_title_id: u64,
    new_title_id: u64,
    flags: u32,
    unknown2: u32,
    unknown_buffer: [u8; 0xE0],
}
const_assert!(core::mem::size_of::<OfficialBoot>() == 0x100);

#[repr(C)]
/// The parameters passed to "Unlaunch" in order to override the default boot option.
pub struct UnlaunchParams {
    /// Flags used to boot the title
    pub flags: UnlaunchBootFlags,
    /// Color to use for the upper screen while booting (usually white or black)
    pub upper_bg_color: u16,
    /// Color to use for the lower screen while booting (usually white or black)
    pub lower_bg_color: u16,
    /// Reserved
    _0x18: [u8; 0x20],
    /// UTF-16 file path to the app we want to boot
    file_path: [u16; 0x104],
    /// Reserved
    _0x240: [u8; 0x1c0],
}
bitflags::bitflags! {
    /// Flags for "Unlaunch" to use when booting a new app
    pub struct UnlaunchBootFlags: u32 {
        /// Boot the application immediately
        const BOOT = (1<<0);
        /// Use the background colors specified in the parameters
        const USE_BG_COLORS = (1<<1);
    }
}

#[repr(C)]
/// The Boot structure for starting an app via the "Unlaunch" method
pub struct UnlaunchBoot {
    /// Boot signature "AutoLoadInfo"
    signature: [u8; 12],
    /// Length of area to crc (should be the length of the parameters, i.e size_of UnlaunchParams)
    crc_len: u16,
    /// CRC code for the parameters
    crc: u16,
    /// The parameters
    params: core::mem::MaybeUninit<UnlaunchParams>,
}
const_assert!(core::mem::size_of::<UnlaunchBoot>() == 0x400);

impl UnlaunchBoot {
    // Extracts Unlaunch Autoboot parameters
    pub fn parameters<'a>(&'a self) -> Option<&'a UnlaunchParams> {
        // Validate Signature
        if &self.signature != b"AutoLoadInfo" {
            return None;
        }
        // Validate CRC length
        if self.crc_len != core::mem::size_of::<UnlaunchParams>() as u16 {
            return None;
        }
        // Calculate CRC
        // Safety: the length is known and function only reads memory
        let crc = unsafe {
            crate::swi_crc16(
                0xFFFF,
                core::ptr::addr_of!(self.params) as *const (),
                self.crc_len as usize,
            )
        };
        // Verify CRC
        // Safety: params doesn't have any invalid bit patterns and have been verified by the CRC
        (crc == self.crc).then_some(unsafe { self.params.assume_init_ref() })
    }
}
impl UnlaunchParams {
    /// Convert the UTF-16 path in the parameters to a standard rust string.
    /// 
    /// If an error occurs, an empty string is returned instead.
    pub fn parse_path(&self) -> String {
        // NOTE: unwrap_or_default is a code size optimization.
        // We really don't need to know *why* parsing failed.
        String::from_utf16(&self.file_path).unwrap_or_default()
    }
}
/// Boot info provided by past boots of the console
pub const BOOT_INFO: MemoryWrapper<BootMethods> = MemoryWrapper(0x200_0000 as *mut _);

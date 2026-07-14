use alloc::boxed::Box;
use reboot_lib::music_modules::mods::MODHeader;

pub fn send_mod_file(module: Box<MODHeader>) -> Option<Box<MODHeader>> {
    unsafe {
        match reboot_lib::arm9_send_arm7(0, Box::into_raw(module) as *mut ()) {
            Ok(_) => None,
            Err(old_mod) => Some(Box::from_raw(u32::from(old_mod) as *mut MODHeader)),
        }
    }
}
pub fn stop_mod_file() -> Option<Box<MODHeader>> {
    unsafe {
        match reboot_lib::arm9_send_arm7(0, core::ptr::null_mut()) {
            Ok(_) => None,
            Err(old_mod) => Some(Box::from_raw(u32::from(old_mod) as *mut MODHeader)),
        }
    }
}
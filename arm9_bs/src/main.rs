#![no_std]
#![no_main]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _start() {
    unsafe { common::bootstrap::boot_arm9() };
}

#[cfg(target_arch = "arm")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

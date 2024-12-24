#![no_std]
#![no_main]
#[no_mangle]
pub unsafe extern "C" fn _start() {
    unsafe { common::bootstrap::boot_arm7() };
}

#[cfg(target_arch = "arm")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#![no_main]
#![no_std]

#[no_mangle]
pub fn _start() {
    unsafe
    {
        core::ptr::write_volatile(0x4000000 as *mut u32, 0b000000000000000010000000000000000);
        core::ptr::write_volatile(0x5000000 as *mut u16, 0b0000111101010100);
        loop {}
    }
}

use core::panic::PanicInfo;
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

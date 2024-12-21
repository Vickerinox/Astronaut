#![no_main]
#![no_std]
mod swi;

use core::arch::asm;
use reboot_lib::IPC_FIFO_HARDWARE;
#[no_mangle]
pub unsafe extern "C" fn _start() {
    asm!(
        // Set up the stack pointer to 0x7C00
        "ldr sp, =0x037B9FFC",

        // Call the main function
        "bl {main}",

        // Halt the CPU after main returns (if it does)
        "2: b 2b", // Infinite loop

        main = sym main, // Link the `main` symbol
        options(noreturn) // No return possible from this function
    );
}


fn main() {
    unsafe {
        IPC_FIFO_HARDWARE.enable();
        IPC_FIFO_HARDWARE.set_status(0);   
        let mut key = [0u32;4];
        swi::generate_cid_key(&mut key);
        reboot_lib::load_nand_key_x(0);
        reboot_lib::load_nand_key_y(0, &[0x0AB9DC76, 0xBD4DC4D3, 0x202DDD1D, 0xE1A00005]);
        reboot_lib::nand_crypt_init(0);
        let mut buffer: *mut [reboot_lib::StorageSector] = core::slice::from_raw_parts_mut(core::ptr::null_mut(), 0);
        //;
        //IPC_FIFO_HARDWARE.send_raw_blocking(0xDEADBEEF);
        while IPC_FIFO_HARDWARE.read_status() != 0 {}
        loop {
            while IPC_FIFO_HARDWARE.read_status() == 0 {}
            let arg = IPC_FIFO_HARDWARE.recieve_raw_blocking();
            match IPC_FIFO_HARDWARE.read_status() {
                1 => {
                    let controls = !core::ptr::read_volatile(0x4000130 as *const u16);
                    IPC_FIFO_HARDWARE.send_raw_blocking(controls as u32);
                }
                2 => {
                    let aux = IPC_FIFO_HARDWARE.recieve_raw_blocking();
                    buffer = core::slice::from_raw_parts_mut(arg as *mut _, aux as usize);
                }
                3 => {
                    reboot_lib::AES_HARDWARE.mmc_read_decrypt(buffer, &key, arg);
                }
                4 => {
        
                }
                5 => {
        
                }
                6 => {
                    let new_start = arg as *mut extern "C" fn();
                    (*new_start)();
                }
                _ => ()
            }
            IPC_FIFO_HARDWARE.set_status(1);
            while IPC_FIFO_HARDWARE.read_status() != 0 {}
            IPC_FIFO_HARDWARE.set_status(0);
        }
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe {
        IPC_FIFO_HARDWARE.set_status(7);
    }
    loop {}
}

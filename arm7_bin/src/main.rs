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

fn add_on_key(key: &mut [u32; 4], add: u32) {
    let carry;
    let carry2;
    let carry3;
    (key[0], carry) = key[0].overflowing_add(add);
    (key[1], carry2) = key[1].overflowing_add(carry as u32);
    (key[2], carry3) = key[2].overflowing_add(carry2 as u32);
    key[3] = key[3].wrapping_add(carry3 as u32);
}
fn main() {
    unsafe {
        core::ptr::write_volatile(0x200_0000 as *mut u32, 80);
        IPC_FIFO_HARDWARE.enable();
        let nand_buffer = IPC_FIFO_HARDWARE.recieve_raw_blocking();
        let nand_buffer = core::slice::from_raw_parts_mut(nand_buffer as usize as *mut u8, 512);
        IPC_FIFO_HARDWARE.set_status(2);
        let nand_init: Result<(), reboot_lib::Status> = Ok(()); //reboot_lib::init_sdmmc(reboot_lib::DeviceSelect::SDCardSlot);

        match nand_init {
            Ok(()) => {
                let read_mbr = reboot_lib::read_sectors(reboot_lib::DeviceSelect::EMMC, 0, nand_buffer);
                match read_mbr {
                    Ok(_) => IPC_FIFO_HARDWARE.send_raw_blocking(0xDEADBEEF),
                    Err(a) => IPC_FIFO_HARDWARE.send_raw_blocking(a.bits()),
                }
            },
            Err(a) => IPC_FIFO_HARDWARE.send_raw_blocking(a.bits()),
        }
        /*
        let mut key = swi::generate_cid_key();
        add_on_key(&mut key, 1);
        reboot_lib::load_nand_key_x(0);
        reboot_lib::load_nand_key_y(0, &[0x0AB9DC76, 0xBD4DC4D3, 0x202DDD1D, 0xE1A00005]);
        reboot_lib::nand_crypt_init(0);

        let mut result = [0u32; 128];

        reboot_lib::AES_HARDWARE.ctr_crypt_block(&mut result, &key);
        */

        loop {}
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe {
        IPC_FIFO_HARDWARE.set_status(7);
    }
    loop {}
}

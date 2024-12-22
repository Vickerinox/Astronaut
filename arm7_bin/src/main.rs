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
        match reboot_lib::init_sdmmc(reboot_lib::DeviceSelect::SDCardSlot) {
            Ok(()) => IPC_FIFO_HARDWARE.send_raw_blocking(0xDEADBEEF),
            Err(a) => IPC_FIFO_HARDWARE.send_raw_blocking(a.bits()),
        }
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
                    mmc_read_decrypt(buffer, &key, arg);
                }
                4 => {
        
                }
                5 => {
                    sd_read_sectors(buffer, arg);
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
use reboot_lib::AES_HARDWARE;

/// read and decrypt the given sectors from NAND using NDMA.
pub unsafe fn mmc_read_decrypt(data: *mut [reboot_lib::StorageSector], ctr_base: &[u32; 4], sector: u32) -> Result<(), ()> {

        
    let a = reboot_lib::read_sectors(reboot_lib::DeviceSelect::EMMC, sector, core::slice::from_raw_parts_mut(core::ptr::null_mut(), data.len()));
    
    fn add_on_key(key: &mut [u32; 4], add: u32) {
        let carry;
        let carry2;
        let carry3;
        (key[0], carry) = key[0].overflowing_add(add);
        (key[1], carry2) = key[1].overflowing_add(carry as u32);
        (key[2], carry3) = key[2].overflowing_add(carry2 as u32);
        key[3] = key[3].wrapping_add(carry3 as u32);
    }
    let mut key = ctr_base.clone();
    add_on_key(&mut key, sector << 5);

    use reboot_lib::ndma::{Control, NDMA_HARDWARE};
    AES_HARDWARE.master_control.write(0);
    AES_HARDWARE.reset();
    let length = (data.len() << 9) as u32;
    AES_HARDWARE.load_iv(&key);
    AES_HARDWARE.set_block_count((length >> 4) as u16);
    //setup dma 1 to read from the sdmmc fifo, and write to the AES engine input.
    let in_dma = reboot_lib::ndma::ChannelConfig {
        word_count: length >> 2,
        block_size: 4,
        timing: 8,
        fill_mode: 0,
        control: Control::DST_MODE_FIXED
            | Control::SRC_MODE_FIXED
            | Control::BLOCK_SIZE_4
            | Control::START_ARM7_WRITE_AES
            | Control::ENABLE,
    };
    NDMA_HARDWARE.set_raw_dma(1, in_dma, 0x400490C as _, 0x4004408 as _);
    //setup dma 0 to read from the AES engine output, and write to the provided buffer
    let out_dma = reboot_lib::ndma::ChannelConfig {
        word_count: length >> 2,
        block_size: 4,
        timing: 8,
        fill_mode: 0,
        control: Control::SRC_MODE_FIXED
            | Control::DST_MODE_INCREMENT
            | Control::BLOCK_SIZE_4
            | Control::START_ARM7_READ_AES
            | Control::ENABLE,
    };
    NDMA_HARDWARE.set_raw_dma(0, out_dma, 0x400440C as _, data as *mut () as _);
    //start the AES engine (starting the DMA transfers)
    AES_HARDWARE.start((0 << 14) | (3 << 12) | (2 << 28));
    
    //await for everything to finish
    NDMA_HARDWARE.await_channel(0);
    NDMA_HARDWARE.await_channel(1);
    AES_HARDWARE.wait_aes_busy();
    match a {
        Ok(_) => Ok(()),
        Err(_) => Err(()),
    }
}


/// read from the SD card using NDMA.
pub unsafe fn sd_read_sectors(data: *mut [reboot_lib::StorageSector], sector: u32) -> Result<(), ()> {
    use reboot_lib::ndma::{Control, NDMA_HARDWARE};

    let a = reboot_lib::read_sectors(reboot_lib::DeviceSelect::SDCardSlot, sector, data);
    match a {
        Ok(_) => (),
        Err(_) => return Err(()),
    }
    //await for everything to finish
    NDMA_HARDWARE.await_channel(0);
    Ok(())
}

pub unsafe fn nocash_write(str: &str) {
    const NOCASH_OUT_CHR: *mut u8 = 0x4fffa1c as *mut u8;
    for byte in str.as_bytes() {
        NOCASH_OUT_CHR.write_volatile(*byte);
    }
}
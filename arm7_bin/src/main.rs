#![feature(ptr_metadata)]
#![no_main]
#![no_std]
mod swi;
mod mmc;

use core::arch::asm;
use reboot_lib::{
    spi::{Control, PowerRegiser},
    IPC_FIFO_HARDWARE, MMC_CONTROLLER,
};

//use crate::mmc::NAND_DEVICE;
const DSI_WRAM_START: usize = 0x037B8000;
#[no_mangle]
pub unsafe extern "C" fn _start() {
    asm!(
        //turn off interrupts via the IME register
        "mov r0, #0x04000000",
        "str r0, [r0, #0x208]",

        //load start of stack(s)
        "mov r0, #0x12",
        "msr cpsr, r0",
        "ldr sp, ={stack_irq}",

        "mov r0, #0x13",
        "msr cpsr, r0",
        "ldr sp, ={stack_svc}",

        "mov r0, #0x1F",
        "msr cpsr, r0",
        "ldr sp, ={stack_sys}",

        // Call the main function
        "bl {main}",

        // Halt the CPU after main returns (if it does)
        "2: b 2b", // Infinite loop

        stack_irq = const DSI_WRAM_START + 0x1000,
        stack_svc = const DSI_WRAM_START + 0x2000,
        stack_sys = const DSI_WRAM_START + 0x3000,

        main = sym main, // Link the `main` symbol
        options(noreturn) // No return possible from this function
    );
}


static mut FRAME_COUNTER: u32 = 0;
fn vblank_interrupt() {
    unsafe {FRAME_COUNTER += 1};
}
fn main() {
    unsafe {
        IPC_FIFO_HARDWARE.enable();

        reboot_lib::sound::SOUND_HARDWARE.init();
        reboot_lib::sound::SOUND_HARDWARE.channels[8].start_test_beep();
        reboot_lib::init_interrupts();
        reboot_lib::spi::touchscreen::init_tsc();
        reboot_lib::i2c::init();
        reboot_lib::spi::write_powerman(PowerRegiser::Control(Control::ENABLE_BACKLIGHTS | Control::ENABLE_SOUND_AMP));

        (0x400_0008 as *mut u32)
            .write_volatile((0x400_0008 as *const u32).read_volatile() | (1 << 17));
        (0x400_0004 as *mut u32)
            .write_volatile((0x400_0004 as *const u32).read_volatile() | (1 << 2));

        let mut key = [0u32; 4];
        swi::generate_cid_key(&mut key);
        reboot_lib::load_nand_key_x(0);
        reboot_lib::load_nand_key_y(0, &[0x0AB9DC76, 0xBD4DC4D3, 0x202DDD1D, 0xE1A00005]);
        reboot_lib::nand_crypt_init(0);

        let mut buffer: *mut [reboot_lib::StorageSector] =
            core::slice::from_raw_parts_mut(0x2FFFE00 as *mut reboot_lib::StorageSector, 1);

        reboot_lib::IPC_FIFO_HARDWARE.set_status(1);
        while reboot_lib::IPC_FIFO_HARDWARE.read_status() != 1 {}
        reboot_lib::IPC_FIFO_HARDWARE.set_status(0);


        reboot_lib::init_sdmmc(reboot_lib::DeviceSelect::EMMC);
        let send = match reboot_lib::init_sdmmc(reboot_lib::DeviceSelect::EMMC) {
            Ok(_) => 1,
            Err(_) => 0,
        };
        
        
        IPC_FIFO_HARDWARE.send_raw_blocking(send);
        
        
        loop {
            while IPC_FIFO_HARDWARE.recv_fifo_empty() {}
            let mut response = 0;
            match IPC_FIFO_HARDWARE.recieve_raw_blocking() {
                1 => {
                    let Some([0]) = gather_args() else { response = 0x8000_0000; continue;};
                    let controls = !core::ptr::read_volatile(0x4000130 as *const u16);
                    let mut controls = reboot_lib::Buttons::from_bits_retain(controls);
                    if !reboot_lib::spi::touchscreen::is_pen_down() {
                        controls ^= reboot_lib::Buttons::PEN_DOWN;
                    }
                    response = controls.bits() as u32;
                }
                2 => {
                    let Some([ptr, len]) = gather_args() else { response = 0x8000_0000; continue;};
                    buffer = core::slice::from_raw_parts_mut(ptr as *mut _, len as usize);
                }
                3 => {
                    let Some([arg]) = gather_args() else { response = 0x8000_0000; continue;};
                    response = match mmc_read_decrypt(buffer, &key, arg) {
                        Ok(_) => 0,
                        Err(e) => 0x8000_0000 | e.bits(),
                    };
                }
                4 => {

                }
                5 => {
                    let Some([arg]) = gather_args() else { response = 0x8000_0000; continue;};
                    sd_read_sectors(buffer, arg);
                }
                6 => {
                    let Some([arg]) = gather_args() else { response = 0x8000_0000; continue;};
                    IPC_FIFO_HARDWARE.send_raw_blocking(0);
                    (*(arg as *mut () as *mut unsafe extern fn()))();
                }
                7 => {
                    let Some([arg]) = gather_args() else { response = 0x8000_0000; continue;};
                    firmware_read(buffer, arg);
                }
                _ => {response = 0x8000_0000},
            }
            IPC_FIFO_HARDWARE.send_raw_blocking(response);
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

pub unsafe fn firmware_read(data: *mut [reboot_lib::StorageSector], offset: u32) {
    let (ptr, len) = data.to_raw_parts();
    let buffer = core::slice::from_raw_parts_mut(ptr as *mut u8, len << 9);
    reboot_lib::spi::SPI_HARDWARE.read_firmware(buffer, offset);
}
/// read and decrypt the given sectors from NAND using NDMA.
pub unsafe fn mmc_read_decrypt(
    data: *mut [reboot_lib::StorageSector],
    ctr_base: &[u32; 4],
    sector: u32,
) -> Result<(), reboot_lib::Status> {
    return reboot_lib::read_sectors(reboot_lib::DeviceSelect::EMMC, sector, data);

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
    let ptr = data as *mut ();
    let len = data.len();
    reboot_lib::AES_HARDWARE.ctr_crypt_block(
        0x0380_0000 as *mut _,
        ptr as *mut _,
        (len << 6) as u32,
        &key,
    );
    Ok(())
}

/// read from the SD card using NDMA.
pub unsafe fn sd_read_sectors(
    data: *mut [reboot_lib::StorageSector],
    sector: u32,
) -> Result<(), ()> {
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

unsafe fn gather_args<const N: usize>() -> Option<[u32; N]> {
    let mut array = [0u32; N];
    for data in array.iter_mut() {
        *data = IPC_FIFO_HARDWARE.recieve_raw_blocking();
    }
    Some(array)
}


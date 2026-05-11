#![feature(ptr_metadata)]
#![no_main]
#![no_std]
mod mmc;
mod mmc_new;
mod swi;

use common::bootstrap;
use core::arch::asm;
use reboot_lib::{
    check_sdmmc,
    i2c::I2CRegister,
    ndma::NDMA_HARDWARE,
    sound::SOUND_HARDWARE,
    spi::{
        touchscreen::read_tsc_pos_cdc,
        Control, PowerRegiser,
    },
    timers::TIMERS,
    write_sd_sectors, Status, StorageSector, AES_HARDWARE, DMA_HARDWARE, IPC_FIFO_HARDWARE,
    MMC_CONTROLLER, SDIO_CONTROLLER,
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

pub mod music;
/*
unsafe fn update_volume() {
    match reboot_lib::i2c::I2C_HARDWARE.read_register(reboot_lib::i2c::PowerRegister::VOL) {
        Ok(value) => reboot_lib::sound::SOUND_HARDWARE
            .master_control
            .modify(|i| (i & !0xFF) | (value as u32)),
        Err(_) => (),
    }
}
    */
unsafe fn power_button_interrupt() {
    let irq_cause = unsafe {
        reboot_lib::i2c::I2C_HARDWARE
            .read_register(reboot_lib::i2c::PowerRegister::PWRIF)
            .map(|i| i & 3)
    };
    match irq_cause {
        Ok(1) => {
            unsafe {
                //set warmboot
                reboot_lib::i2c::I2C_HARDWARE
                    .write_register(reboot_lib::i2c::PowerRegister::RESETFLAG, 1);
                //trigger reset
                reboot_lib::i2c::I2C_HARDWARE
                    .write_register(reboot_lib::i2c::PowerRegister::PWRCNT, 1);
            }
        }
        Ok(2) => unsafe {
            reboot_lib::spi::write_powerman(PowerRegiser::Control(Control::SHUT_DOWN_POWER));
        },
        _ => { /* unknown, afaik, seems to mean any other i2c interrupt */ }
    }
}
pub mod init;

fn main() {
    unsafe {
        //start talking to the ARM9 ASAP
        IPC_FIFO_HARDWARE.enable();
        
        //start doing hardware init
        init::init_power_regs();
        init::init_i2c();
        init::init_ntr_sound();
        init::init_powerman();
        init::init_nwram();

        (0x4004C02 as *mut u16).write((1 << 6) << 8);

        /*
        (0x400_0008 as *mut u32)
            .write_volatile((0x400_0008 as *const u32).read_volatile() | (1 << 17));

        (0x400_0004 as *mut u32)
            .write_volatile((0x400_0004 as *const u32).read_volatile() | (1 << 3));
        */

        let mut key = [0u32; 4];
        swi::generate_cid_key(&mut key);

        let console_id: [u32; 2] = [
            (0x4004D00 as *const u32).read_volatile(),
            (0x4004D04 as *const u32).read_volatile(),
        ];
        reboot_lib::load_nand_key_x(3, console_id);
        reboot_lib::load_nand_key_y(3, &[0x0AB9DC76, 0xBD4DC4D3, 0x202DDD1D, 0xE1A00005]);
        reboot_lib::nand_crypt_init(3);

        reboot_lib::spi::touchscreen::init_tsc_dsi();

        reboot_lib::init_interrupts();

        let mut buffer: *mut [reboot_lib::StorageSector] =
            core::slice::from_raw_parts_mut(0x2FF0000 as *mut reboot_lib::StorageSector, 1);

        reboot_lib::IPC_FIFO_HARDWARE.set_status(1);
        while reboot_lib::IPC_FIFO_HARDWARE.read_status() != 1 {}
        reboot_lib::IPC_FIFO_HARDWARE.set_status(0);

        reboot_lib::MMC_CONTROLLER.tmio_init();

        reboot_lib::enable_interrupt(reboot_lib::ARM7Interrupt::IPCNonEmpty);
        reboot_lib::enable_interrupt(reboot_lib::ARM7Interrupt::VBlank);
        IPC_FIFO_HARDWARE.enable_recv_irq();
        let mut firm = [StorageSector::default()];

        firmware_read(&mut firm, 0);
        let settings_offset = u16::from_le_bytes([firm[0].bytes()[0x20], firm[0].bytes()[0x21]]);

        firmware_read(&mut firm, settings_offset as u32 * 8);
        let firm = firm[0].bytes();

        let adcx1 = u16::from_le_bytes([firm[0x158], firm[0x159]]);
        let adcy1 = u16::from_le_bytes([firm[0x15A], firm[0x15B]]);
        let scrx1 = firm[0x15C];
        let scry1 = firm[0x15D];

        let adcx2 = u16::from_le_bytes([firm[0x15E], firm[0x15F]]);
        let adcy2 = u16::from_le_bytes([firm[0x160], firm[0x161]]);
        let scrx2 = firm[0x162];
        let scry2 = firm[0x163];

        let x_scale = ((scrx2 as i32 - scrx1 as i32) << 19) / (adcx2 as i32 - adcx1 as i32);
        let y_scale = ((scry2 as i32 - scry1 as i32) << 19) / (adcy2 as i32 - adcy1 as i32);
        let x_offset =
            (((adcx1 as i32 + adcx2 as i32) * x_scale) - ((scrx1 as i32 + scrx2 as i32) << 19)) / 2;
        let y_offset =
            (((adcy1 as i32 + adcy2 as i32) * y_scale) - ((scry1 as i32 + scry2 as i32) << 19)) / 2;
        /*
        let send = ;
        IPC_FIFO_HARDWARE.send_raw_blocking(send);
        let send = ;
        IPC_FIFO_HARDWARE.send_raw_blocking(send);
        */
        /*
        reboot_lib::set_interrupt_function(
            reboot_lib::ARM7Interrupt::Powerbutton,
            power_button_interrupt,
        );
        reboot_lib::enable_interrupt(reboot_lib::ARM7Interrupt::Powerbutton);
        */
        let mut last_x = 0;
        let mut last_y = 0;
        let mut pen_down = false;
        let mut last_pen = false;
        //reboot_lib::spi::touchscreen::enable_tsc();

        loop {
            while IPC_FIFO_HARDWARE.recv_fifo_empty() {}
            let mut response = 0;
            match IPC_FIFO_HARDWARE.recieve_raw_blocking() {
                1 => {
                    if IPC_FIFO_HARDWARE.recieve_raw_blocking() != 0 {
                        response = 0x8000_0000;
                        continue;
                    };
                    assert!(IPC_FIFO_HARDWARE.recieve_value_raw().is_err());
                    let controls = !core::ptr::read_volatile(0x4000130 as *const u16);
                    let mut controls = reboot_lib::Buttons::from_bits_retain(controls);
                    /*
                    if core::ptr::read_volatile(0x4000136 as *const u16) & (1<<6) == 0 {
                        controls ^= reboot_lib::Buttons::PEN_DOWN;
                    }
                    */
                    //if core::ptr::read_volatile(0x4000136 as *const u16) & (1<<6) == 0 {

                    if reboot_lib::spi::touchscreen::is_pen_down() {
                        if let Some((x, y)) = read_tsc_pos_cdc() {
                            let scr_x = {
                                let x = x as i32 * x_scale - x_offset + x_scale / 2;
                                (x >> 19).clamp(0, 255)
                            };
                            let scr_y = {
                                let y = y as i32 * y_scale - y_offset + y_scale / 2;
                                (y >> 19).clamp(0, 191)
                            };
                            if last_pen {
                                last_x = scr_x as u8;
                                last_y = scr_y as u8;
                            }
                        }
                        if last_pen {
                            pen_down = true;
                        }
                        last_pen = true;
                    } else {
                        if !last_pen {
                            pen_down = false;
                        }
                        last_pen = false;
                    }

                    if !pen_down {
                        controls ^= reboot_lib::Buttons::PEN_DOWN;
                    };

                    response =
                        controls.bits() as u32 | ((last_x as u32) << 16) | ((last_y as u32) << 24);
                }
                2 => {
                    let ptr = IPC_FIFO_HARDWARE.recieve_raw_blocking();
                    let len = IPC_FIFO_HARDWARE.recieve_raw_blocking();
                    assert!(IPC_FIFO_HARDWARE.recieve_value_raw().is_err());
                    buffer = core::slice::from_raw_parts_mut(ptr as *mut _, len as usize);
                }
                3 => {
                    let arg = IPC_FIFO_HARDWARE.recieve_raw_blocking();
                    assert!(IPC_FIFO_HARDWARE.recieve_value_raw().is_err());
                    response = match mmc_read_decrypt(buffer, &key, arg) {
                        Ok(_) => 0,
                        Err(e) => 0x8000_0000 | e.bits(),
                    };
                }
                11 => {
                    let arg = IPC_FIFO_HARDWARE.recieve_raw_blocking();
                    assert!(IPC_FIFO_HARDWARE.recieve_value_raw().is_err());
                    response = match arg {
                        1 => check_sdmmc(reboot_lib::DeviceSelect::SDCardSlot).bits(),
                        2 => check_sdmmc(reboot_lib::DeviceSelect::EMMC).bits(),
                        _ => 1,
                    }
                }
                5 => {
                    let arg = IPC_FIFO_HARDWARE.recieve_raw_blocking();
                    assert!(IPC_FIFO_HARDWARE.recieve_value_raw().is_err());

                    response = match sd_read_sectors(buffer, arg) {
                        Ok(_) => 0,
                        Err(e) => e.bits(),
                    }
                }
                10 => {
                    let arg = IPC_FIFO_HARDWARE.recieve_raw_blocking();
                    assert!(IPC_FIFO_HARDWARE.recieve_value_raw().is_err());
                    response = match write_sd_sectors(arg, buffer) {
                        Ok(_) => 0,
                        Err(e) => e.bits(),
                    }
                }

                6 => {
                    let _arg = IPC_FIFO_HARDWARE.recieve_raw_blocking();
                    assert!(IPC_FIFO_HARDWARE.recieve_value_raw().is_err());
                    IPC_FIFO_HARDWARE.send_raw_blocking(common::bootstrap::boot_arm9 as *const () as u32);
                    reboot_lib::disable_all_interrupts();
                    SOUND_HARDWARE.init();
                    AES_HARDWARE.init_from_header(
                        &*(common::bootstrap::BOOTLOADER_MEM
                            as *const common::bootstrap::HeaderTWL),
                        console_id,
                    );
                    TIMERS.clear();
                    DMA_HARDWARE.reset();
                    NDMA_HARDWARE.reset();
                    MMC_CONTROLLER.reset();
                    SDIO_CONTROLLER.reset();
                    reboot_lib::i2c::I2C_HARDWARE.write_register(
                        I2CRegister::I2cPower(reboot_lib::i2c::PowerRegister::MMCPWR),
                        0,
                    );
                    bootstrap::boot_arm7();
                }
                7 => {
                    let arg = IPC_FIFO_HARDWARE.recieve_raw_blocking();
                    firmware_read(buffer, arg);
                }

                8 => {
                    let arg = IPC_FIFO_HARDWARE.recieve_raw_blocking();
                    response = match arg {
                        1 => match reboot_lib::init_sdmmc(reboot_lib::DeviceSelect::SDCardSlot) {
                            Ok(_) => 0,
                            Err(err) => err as u16 as u32,
                        },
                        2 => match reboot_lib::init_sdmmc(reboot_lib::DeviceSelect::EMMC) {
                            Ok(_) => 0,
                            Err(err) => err as u16 as u32,
                        },
                        _ => 0x8000_0000,
                    }
                }
                9 => {
                    let module_type = IPC_FIFO_HARDWARE.recieve_raw_blocking();
                    let pointer = IPC_FIFO_HARDWARE.recieve_raw_blocking();

                    match module_type {
                        0 => music::set_mod(pointer as *mut _),
                        1 => {
                            //music::set_procedural();
                        }
                        _ => response = 0x8000_0000,
                    }
                }
                12 => {
                    let _arg = IPC_FIFO_HARDWARE.recieve_raw_blocking();
                    assert!(IPC_FIFO_HARDWARE.recieve_value_raw().is_err());

                    AES_HARDWARE.init_from_header(
                        &*(common::bootstrap::BOOTLOADER_MEM
                            as *const common::bootstrap::HeaderTWL),
                        console_id,
                    );

                    let header = &(*common::bootstrap::BOOTINFO_MEM).twl_header;


                    response = 0x8000_0000;

                    if header.arm9i_offset == header.modcrypt1_offset || header.arm9_offset == header.modcrypt1_offset {
                        if header.modcrypt2_len == 0 {
                            response = 0
                        } else {
                            if header.arm7i_offset == header.modcrypt2_offset {
                                response = 0;
                            } else {
                                response = 2;
                            }
                        } 
                    } else {
                        response = 1;
                    }

                    //response = 0x8000_0000;
                    
                    if response == 0 {

                        
                        

                        let key: [u32; 4] = core::array::from_fn(|i| header.arm9i_sha1[i]);
                        let ptr = header.arm9i_load;
                        let len = header.modcrypt1_len;
                        AES_HARDWARE.reset();
                        AES_HARDWARE.reset();
                        AES_HARDWARE.wait_key_busy();
                        AES_HARDWARE.set_key_slot(0);
                        AES_HARDWARE.wait_key_busy();
                        reboot_lib::AES_HARDWARE.ctr_crypt_block(
                            core::slice::from_raw_parts_mut(ptr as *mut _, len as usize),
                            &key,
                        );

                        

                        if header.modcrypt2_len > 0 {    
                            let key: [u32; 4] = core::array::from_fn(|i| header.arm7i_sha1[i]);
                            let ptr = header.arm7i_load;
                            let len = header.modcrypt2_len;
                            AES_HARDWARE.reset();
                            AES_HARDWARE.reset();
                            AES_HARDWARE.wait_key_busy();
                            AES_HARDWARE.set_key_slot(0);
                            AES_HARDWARE.wait_key_busy();
                            reboot_lib::AES_HARDWARE.ctr_crypt_block(
                                core::slice::from_raw_parts_mut(ptr as *mut _, len as usize),
                                &key,
                            );
                        }

                    }
                }
                _ => response = 0x8000_0000,
            }
            IPC_FIFO_HARDWARE.send_raw_blocking(response);
        }
    }
}

#[cfg(target_arch = "arm")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe {
        (0x400_0208 as *mut u32).write_volatile(0);
        IPC_FIFO_HARDWARE.set_status(7);
    }
    loop {}
}
pub unsafe fn clear_arm7_regs() {
    (0x04000004 as *mut u16).write_volatile(0);
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
    reboot_lib::read_sectors(reboot_lib::DeviceSelect::EMMC, sector, data)?;

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
    AES_HARDWARE.set_key_slot(3);
    reboot_lib::AES_HARDWARE.ctr_crypt_block(
        core::slice::from_raw_parts_mut(ptr as *mut _, len << 7),
        &key,
    );
    Ok(())
}

/// read from the SD card using NDMA.
pub unsafe fn sd_read_sectors(
    data: *mut [reboot_lib::StorageSector],
    sector: u32,
) -> Result<(), Status> {
    reboot_lib::read_sectors(reboot_lib::DeviceSelect::SDCardSlot, sector, data)
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

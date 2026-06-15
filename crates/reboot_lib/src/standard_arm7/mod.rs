mod mmc;
mod mmc_new;
mod swi;

use crate::{
    check_sdmmc,
    i2c::I2CRegister,
    ndma::NDMA_HARDWARE,
    sound::SOUND_HARDWARE,
    spi::{touchscreen::read_tsc_pos_cdc, Control, PowerRegiser, SPI_HARDWARE},
    timers::TIMERS,
    write_sd_sectors, AESCnt, Status, StorageSector, AES_HARDWARE, DMA_HARDWARE, IPC_FIFO_HARDWARE,
    MMC_CONTROLLER, SDIO_CONTROLLER,
};
use common::bootstrap::{self, BOOTINFO_MEM};
use core::arch::asm;

pub mod music;
/*
unsafe fn update_volume() {
    match crate::i2c::I2C_HARDWARE.read_register(crate::i2c::PowerRegister::VOL) {
        Ok(value) => crate::sound::SOUND_HARDWARE
            .master_control
            .modify(|i| (i & !0xFF) | (value as u32)),
        Err(_) => (),
    }
}
    */
unsafe fn power_button_interrupt() {
    let irq_cause = unsafe {
        crate::i2c::I2C_HARDWARE
            .read_register(crate::i2c::PowerRegister::PWRIF.into())
            .map(|i| i & 3)
    };
    match irq_cause {
        Ok(1) => {
            unsafe {
                //set warmboot
                crate::i2c::I2C_HARDWARE
                    .write_register(crate::i2c::PowerRegister::RESETFLAG.into(), 1);
                //trigger reset
                crate::i2c::I2C_HARDWARE
                    .write_register(crate::i2c::PowerRegister::PWRCNT.into(), 1);
            }
        }
        Ok(2) => unsafe {
            crate::spi::write_powerman(PowerRegiser::Control(Control::SHUT_DOWN_POWER));
        },
        _ => { /* unknown, afaik, seems to mean any other i2c interrupt */ }
    }
}
pub mod init;

pub fn main_arm7() {
    unsafe {
        //start talking to the ARM9 ASAP
        IPC_FIFO_HARDWARE.enable();

        //start doing hardware init
        init::init_power_regs();
        init::init_i2c();
        init::init_ntr_sound();
        init::init_powerman2();
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
        crate::load_nand_key_x(3, console_id);
        crate::load_nand_key_y(3, &[0x0AB9DC76, 0xBD4DC4D3, 0x202DDD1D, 0xE1A00005]);
        crate::nand_crypt_init(3);

        crate::spi::touchscreen::init_tsc_dsi();

        crate::init_interrupts();

        let mut buffer: *mut [crate::StorageSector] =
            core::slice::from_raw_parts_mut(0x2FF0000 as *mut crate::StorageSector, 1);

        crate::IPC_FIFO_HARDWARE.set_status(1);
        while crate::IPC_FIFO_HARDWARE.read_status() != 1 {}
        crate::IPC_FIFO_HARDWARE.set_status(0);

        crate::MMC_CONTROLLER.tmio_init();

        crate::enable_interrupt(crate::ARM7Interrupt::IPCNonEmpty);
        crate::enable_interrupt(crate::ARM7Interrupt::VBlank);
        IPC_FIFO_HARDWARE.enable_recv_irq();

        let mut location = [0u8; 2];
        SPI_HARDWARE.read_firmware(&mut location, 0x20);
        let settings_offset = (u16::from_le_bytes(location) as u32) * 8;

        let mut ctr1 = [0u8];
        SPI_HARDWARE.read_firmware(&mut ctr1, settings_offset + 0x70);
        let [ctr1] = ctr1;

        let mut ctr2 = [0u8];
        SPI_HARDWARE.read_firmware(&mut ctr2, settings_offset + 0x170);
        let [ctr2] = ctr2;

        let mut wifi_ver = [0u8];
        SPI_HARDWARE.read_firmware(&mut wifi_ver, 0x1FD);
        let [wifi_ver] = wifi_ver;

        let offset = if (ctr1 & 0x7f) == ((ctr2 + 1) & 0x7f) {
            settings_offset
        } else {
            settings_offset + 0x100
        };
        let firm_buffer = &mut (*BOOTINFO_MEM).ntr.firmware_data;
        let (user, remainder) = firm_buffer.split_at_mut(0x74);
        SPI_HARDWARE.read_firmware(user, offset);
        let (mac, remainder) = remainder.split_at_mut(6);
        SPI_HARDWARE.read_firmware(mac, 0x36);
        remainder[0] = 0x41;
        remainder[1] = 0x10;
        remainder[0xE8 - 0x7A..(0xEC - 0x7A) + 4].copy_from_slice(&[0x3E, 0, 0, 0, 0, 0, 0, 0]);
        remainder[0xF0 - 0x7A] = 2;
        remainder[0xFF - 0x7A] = wifi_ver;
        let adcx1 = u16::from_le_bytes([user[0x58], user[0x59]]);
        let adcy1 = u16::from_le_bytes([user[0x5A], user[0x5B]]);
        let scrx1 = user[0x5C];
        let scry1 = user[0x5D];

        let adcx2 = u16::from_le_bytes([user[0x5E], user[0x5F]]);
        let adcy2 = u16::from_le_bytes([user[0x60], user[0x61]]);
        let scrx2 = user[0x62];
        let scry2 = user[0x63];

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
        crate::set_interrupt_function(
            crate::ARM7Interrupt::Powerbutton,
            power_button_interrupt,
        );
        crate::enable_interrupt(crate::ARM7Interrupt::Powerbutton);
        */
        let mut last_x = 0;
        let mut last_y = 0;
        let mut pen_down = false;
        let mut last_pen = false;

        crate::twl_wifi::nwifi_init_complete();
        //crate::spi::touchscreen::enable_tsc();

        loop {
            while IPC_FIFO_HARDWARE.recv_fifo_empty() {}
            let mut response = 0;
            match IPC_FIFO_HARDWARE.recieve_raw_blocking() {
                1 => {
                    if IPC_FIFO_HARDWARE.recieve_raw_blocking() != 0 {
                        //response = 0x8000_0000;
                        continue;
                    };
                    assert!(IPC_FIFO_HARDWARE.recieve_value_raw().is_err());
                    let controls = !core::ptr::read_volatile(0x4000130 as *const u16);
                    let mut controls = crate::Buttons::from_bits_retain(controls);
                    /*
                    if core::ptr::read_volatile(0x4000136 as *const u16) & (1<<6) == 0 {
                        controls ^= crate::Buttons::PEN_DOWN;
                    }
                    */
                    //if core::ptr::read_volatile(0x4000136 as *const u16) & (1<<6) == 0 {

                    if crate::spi::touchscreen::is_pen_down() {
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
                        controls ^= crate::Buttons::PEN_DOWN;
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
                        1 => Status::EMPTY.bits(), //check_sdmmc(crate::DeviceSelect::SDCardSlot).bits(),
                        2 => Status::EMPTY.bits(), //check_sdmmc(crate::DeviceSelect::EMMC).bits(),
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
                    IPC_FIFO_HARDWARE
                        .send_raw_blocking(common::bootstrap::boot_arm9 as *const () as u32);
                    crate::disable_all_interrupts();
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
                    let _ = crate::i2c::I2C_HARDWARE.write_register(
                        I2CRegister::I2cPower(crate::i2c::PowerRegister::MMCPWR),
                        0,
                    );

                    core::arch::asm!("mov r11, r11");
                    bootstrap::boot_arm7();
                }
                7 => {
                    let _arg = IPC_FIFO_HARDWARE.recieve_raw_blocking();
                    response = 0x80000000
                    //firmware_read(buffer, arg);
                }

                8 => {
                    let arg = IPC_FIFO_HARDWARE.recieve_raw_blocking();
                    response = match arg {
                        1 => match crate::init_sdmmc(crate::DeviceSelect::SDCardSlot) {
                            Ok(_) => 0,
                            Err(err) => err as u16 as u32,
                        },
                        2 => match crate::init_sdmmc(crate::DeviceSelect::EMMC) {
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

                    let header = &(*common::bootstrap::BOOTINFO_MEM).twl_header;

                    response = 0;

                    /*
                    if core::slice::from_raw_parts(header.arm9_load as *mut u32, 2) != &[0xE7FFDEFF; 2] {
                        response = 10;
                    }
                    */

                    AES_HARDWARE.init_from_header(header, console_id);

                    if header.arm9i_offset != header.modcrypt1_offset
                        && header.head.arm9_offset != header.modcrypt1_offset
                    {
                        response = 1;
                    }
                    if header.modcrypt2_len != 0 {
                        if header.arm7i_offset != header.modcrypt2_offset {
                            response = 2;
                        }
                    }

                    //response = 0x8000_0000;

                    if response == 0 {
                        let ptr = if header.arm9i_offset == header.modcrypt1_offset {
                            header.arm9i_load
                        } else {
                            header.head.arm9_load
                        };

                        let mut key: [u32; 4] = core::array::from_fn(|i| header.arm9_sha1[i]);

                        let len = header.modcrypt1_len;

                        use crate::ndma::Control;

                        let mem =
                            core::slice::from_raw_parts_mut(ptr as *mut u32, len as usize >> 2);

                        decrypt_module(mem, key);
                        if header.modcrypt2_len > 0 {
                            let key: [u32; 4] = core::array::from_fn(|i| header.arm7_sha1[i]);
                            let ptr = header.arm7i_load;
                            let len = header.modcrypt2_len;
                            let mem =
                                core::slice::from_raw_parts_mut(ptr as *mut u32, len as usize >> 2);
                            decrypt_module(mem, key);
                            /*
                            AES_HARDWARE.reset();
                            AES_HARDWARE.reset();
                            AES_HARDWARE.wait_key_busy();
                            AES_HARDWARE.set_key_slot(0);
                            AES_HARDWARE.wait_key_busy();
                            core::arch::asm!("mov r11, r11");
                            AES_HARDWARE.master_control.write(AESCnt::empty());
                            AES_HARDWARE.reset();
                            AES_HARDWARE.load_iv(&key);
                            AES_HARDWARE.payload_blocks.write(len as u16 >> 4);


                            let in_dma = crate::ndma::ChannelConfig {
                                word_count: len >> 2,
                                block_size: 4,
                                timing: 8,
                                fill_mode: 0,
                                control: Control::DST_MODE_FIXED
                                    | Control::SRC_MODE_INCREMENT
                                    | Control::BLOCK_SIZE_4
                                    | Control::START_ARM7_WRITE_AES
                                    | Control::ENABLE,
                            };
                            NDMA_HARDWARE.set_raw_dma(1, in_dma, ptr as _, 0x4004408 as _);
                            let out_dma = crate::ndma::ChannelConfig {
                                word_count: len >> 2,
                                block_size: 4,
                                timing: 8,
                                fill_mode: 0,
                                control: Control::SRC_MODE_FIXED
                                    | Control::DST_MODE_INCREMENT
                                    | Control::BLOCK_SIZE_4
                                    | Control::START_ARM7_READ_AES
                                    | Control::ENABLE,
                            };
                            NDMA_HARDWARE.set_raw_dma(0, out_dma, 0x400440C as _, ptr as _);

                            AES_HARDWARE.start((0 << 14) | (3 << 12) | (2 << 28));

                            NDMA_HARDWARE.await_channel(0);

                            AES_HARDWARE.wait_aes_busy();
                            core::arch::asm!("mov r11, r11");
                            */
                        }
                    }
                }
                _ => response = 0x8000_0000,
            }
            IPC_FIFO_HARDWARE.send_raw_blocking(response);
        }
    }
}

pub unsafe fn decrypt_module(mut mem: &mut [u32], mut key: [u32; 4]) {
    AES_HARDWARE.master_control.write(AESCnt::empty());
    AES_HARDWARE.reset();
    AES_HARDWARE.reset();
    AES_HARDWARE.wait_key_busy();
    AES_HARDWARE.set_key_slot(0);
    AES_HARDWARE.wait_key_busy();

    let mut offset = 0;
    while !mem.is_empty() {
        let split = (0xFFFF * 4).min(mem.len());
        let (chunk, remainder) = mem.split_at_mut(split);
        mem = remainder;
        use crate::ndma::Control;
        let ptr = core::ptr::addr_of_mut!(*chunk);

        let in_dma = crate::ndma::ChannelConfig {
            word_count: split as _,
            block_size: 4,
            timing: 8,
            fill_mode: 0,
            control: Control::DST_MODE_FIXED
                | Control::SRC_MODE_INCREMENT
                | Control::BLOCK_SIZE_4
                | Control::START_ARM7_WRITE_AES
                | Control::ENABLE,
        };

        let out_dma = crate::ndma::ChannelConfig {
            word_count: split as _,
            block_size: 4,
            timing: 8,
            fill_mode: 0,
            control: Control::SRC_MODE_FIXED
                | Control::DST_MODE_INCREMENT
                | Control::BLOCK_SIZE_4
                | Control::START_ARM7_READ_AES
                | Control::ENABLE,
        };
        AES_HARDWARE.master_control.write(AESCnt::empty());
        AES_HARDWARE.reset();
        AES_HARDWARE.load_iv(&key);
        add_on_key(&mut key, (split >> 2) as _);
        AES_HARDWARE.payload_blocks.write((split >> 2) as u16);
        NDMA_HARDWARE.set_raw_dma(1, in_dma, ptr as _, 0x4004408 as _);
        NDMA_HARDWARE.set_raw_dma(0, out_dma, 0x400440C as _, ptr as _);
        AES_HARDWARE.start((0 << 14) | (3 << 12) | (2 << 28) | (1 << 31));
        NDMA_HARDWARE.await_channel(0);
        AES_HARDWARE.wait_aes_busy();
    }
}

pub unsafe fn clear_arm7_regs() {
    (0x04000004 as *mut u16).write_volatile(0);
}
pub unsafe fn firmware_read(data: *mut [crate::StorageSector], offset: u32) {
    let (ptr, len) = data.to_raw_parts();
    let buffer = core::slice::from_raw_parts_mut(ptr as *mut u8, len << 9);
    SPI_HARDWARE.read_firmware(buffer, offset);
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
/// read and decrypt the given sectors from NAND using NDMA.
pub unsafe fn mmc_read_decrypt(
    data: *mut [crate::StorageSector],
    ctr_base: &[u32; 4],
    sector: u32,
) -> Result<(), crate::Status> {
    crate::read_sectors(crate::DeviceSelect::EMMC, sector, data)?;

    let mut key = ctr_base.clone();
    add_on_key(&mut key, sector << 5);
    let ptr = data as *mut ();
    let len = data.len();
    //AES_HARDWARE.set_key_slot(3);

    AES_HARDWARE.master_control.write(AESCnt::empty());
    AES_HARDWARE.reset();
    AES_HARDWARE.reset();
    AES_HARDWARE.wait_key_busy();
    AES_HARDWARE.set_key_slot(3);
    AES_HARDWARE.wait_key_busy();

    crate::AES_HARDWARE.ctr_crypt_block(
        core::slice::from_raw_parts_mut(ptr as *mut _, len << 7),
        &key,
    );
    Ok(())
}

/// read from the SD card using NDMA.
pub unsafe fn sd_read_sectors(
    data: *mut [crate::StorageSector],
    sector: u32,
) -> Result<(), Status> {
    crate::read_sectors(crate::DeviceSelect::SDCardSlot, sector, data)
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

// SPDX-FileCopyrightText: 2026 Viktor Karlsson <viktor@koda.re>
// SPDX-License-Identifier: MIT

mod swi;

use crate::{
    check_sdmmc,
    i2c::I2CRegister,
    ndma::NDMA_HARDWARE,
    sound::{SoundControl, SoundFormat, SOUND_HARDWARE},
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
pub struct Controller {
    last_pen: bool,
    pen_down: bool,
    last_x: u8,
    last_y: u8,
    x_scale: i32,
    x_offset: i32,
    y_scale: i32,
    y_offset: i32,
}
impl Controller {
    unsafe fn next_fetch(&mut self) -> u32 {
        let Self {
            last_pen,
            pen_down,
            last_x,
            last_y,
            x_scale,
            x_offset,
            y_scale,
            y_offset,
        } = self;
        let mut controls = (!core::ptr::read_volatile(0x4000130 as *const u16)) & 0x3FF;
        let controls_2 = (!core::ptr::read_volatile(0x4000136 as *const u16));
        controls |= (controls_2 & 3) << 10;

        let mut controls = crate::Buttons::from_bits_truncate(controls);

        /*
        if core::ptr::read_volatile(0x4000136 as *const u16) & (1<<6) == 0 {
            controls ^= crate::Buttons::PEN_DOWN;
        }
        */
        //if core::ptr::read_volatile(0x4000136 as *const u16) & (1<<6) == 0 {

        if crate::spi::touchscreen::is_pen_down() {
            if let Some((x, y)) = read_tsc_pos_cdc() {
                let scr_x = {
                    let x = x as i32 * *x_scale - *x_offset + *x_scale / 2;
                    (x >> 19).clamp(0, 255)
                };
                let scr_y = {
                    let y = y as i32 * *y_scale - *y_offset + *y_scale / 2;
                    (y >> 19).clamp(0, 191)
                };
                if *last_pen {
                    *last_x = scr_x as u8;
                    *last_y = scr_y as u8;
                }
            }
            if *last_pen {
                *pen_down = true;
            }
            *last_pen = true;
        } else {
            if !*last_pen {
                *pen_down = false;
            }
            *last_pen = false;
        }

        if *pen_down {
            controls ^= crate::Buttons::PEN_DOWN;
        };

        controls.bits() as u32 | ((*last_x as u32) << 16) | ((*last_y as u32) << 24)
    }
}
struct ModCryptor {
    console_id: [u32; 2],
}
#[repr(u32)]
enum ModCryptResult {
    Ok = 0,
    Module1Error = 1,
    Module2Error = 2,
}
impl ModCryptor {
    unsafe fn decrypt_module_ndma(mut mem: &mut [u32], mut key: [u32; 4]) {
        AES_HARDWARE.master_control.write(AESCnt::empty());
        AES_HARDWARE.reset();
        AES_HARDWARE.reset();
        AES_HARDWARE.wait_key_busy();
        AES_HARDWARE.set_key_slot(0);
        AES_HARDWARE.wait_key_busy();

        let mut offset = 0;
        while !mem.is_empty() {
            // Split out the current available chunk
            let split = (0xFFFF * 4).min(mem.len()); // max 0xFFFF * 16 bytes per block
            let (chunk, remainder) = mem.split_at_mut(split);
            mem = remainder;

            // Create NDMA Config
            use crate::ndma::NDMAControl;
            let ptr = core::ptr::addr_of_mut!(*chunk);

            let in_dma = crate::ndma::ChannelConfig {
                word_count: split as _,
                block_size: 4,
                timing: 8,
                fill_mode: 0,
                control: NDMAControl::DST_MODE_FIXED
                    | NDMAControl::SRC_MODE_INCREMENT
                    | NDMAControl::BLOCK_SIZE_4
                    | NDMAControl::START_ARM7_WRITE_AES
                    | NDMAControl::ENABLE,
            };

            let out_dma = crate::ndma::ChannelConfig {
                word_count: split as _,
                block_size: 4,
                timing: 8,
                fill_mode: 0,
                control: NDMAControl::SRC_MODE_FIXED
                    | NDMAControl::DST_MODE_INCREMENT
                    | NDMAControl::BLOCK_SIZE_4
                    | NDMAControl::START_ARM7_READ_AES
                    | NDMAControl::ENABLE,
            };
            // Setup AES
            AES_HARDWARE.master_control.write(AESCnt::empty());
            AES_HARDWARE.reset();
            AES_HARDWARE.load_iv(&key);
            add_on_key(&mut key, (split >> 2) as _);
            AES_HARDWARE.payload_blocks.write((split >> 2) as u16);
            // Setup NDMA
            NDMA_HARDWARE.set_raw_dma(1, in_dma, ptr as _, 0x4004408 as _);
            NDMA_HARDWARE.set_raw_dma(0, out_dma, 0x400440C as _, ptr as _);
            // Start!
            AES_HARDWARE.start((0 << 14) | (3 << 12) | (2 << 28) | (1 << 31));
            NDMA_HARDWARE.await_channel(0);
            AES_HARDWARE.wait_aes_busy();
            // repeat for remaining chunks...
        }
    }
    unsafe fn dewit(&mut self) -> ModCryptResult {
        let Self { console_id } = self;
        let header = &(*common::bootstrap::BOOTINFO_MEM).twl_header;

        AES_HARDWARE.init_from_header(header, console_id.clone());
        if header.modcrypt1_len > 0 && header.head.ntr_rom_size != header.modcrypt1_offset {
            if header.arm9i_offset != header.modcrypt1_offset
                && header.head.arm9_offset != header.modcrypt1_offset
            {
                return ModCryptResult::Module1Error;
            }

            let mut key: [u32; 4] = core::array::from_fn(|i| header.arm9_sha1[i]);

            let mem = {
                let ptr = if header.arm9i_offset == header.modcrypt1_offset {
                    header.arm9i_load
                } else {
                    header.head.arm9_load
                };

                let len = header.modcrypt1_len;

                core::slice::from_raw_parts_mut(ptr as *mut u32, len as usize >> 2)
            };

            Self::decrypt_module_ndma(mem, key);
        }
        if header.modcrypt2_len > 0 && header.head.ntr_rom_size != header.modcrypt2_offset {
            if header.arm7i_offset != header.modcrypt2_offset {
                return ModCryptResult::Module2Error;
            }
            let key: [u32; 4] = core::array::from_fn(|i| header.arm7_sha1[i]);
            let mem = {
                let ptr = header.arm7i_load;
                let len = header.modcrypt2_len;

                core::slice::from_raw_parts_mut(ptr as *mut u32, len as usize >> 2)
            };
            Self::decrypt_module_ndma(mem, key);
        }
        ModCryptResult::Ok 
    }
}
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

        let mut key = [0u32; 4];
        swi::generate_cid_key(&mut key);

        let console_id: [u32; 2] = [
            (0x4004D00 as *const u32).read(),
            (0x4004D04 as *const u32).read(),
        ];
        //in a lot of cases, this *will* already be initialized, (for example after bootstage 1 and 2 on DSi) however
        //seing as there are cases where you most definetely want to initialize it *anyway*, its just made optional.
        #[cfg(feature = "init_nand_aes")]
        {
            crate::load_nand_key_x(3, console_id);
            crate::load_nand_key_y(3, &[0x0AB9DC76, 0xBD4DC4D3, 0x202DDD1D, 0xE1A00005]);
            crate::nand_crypt_init(3);
        }

        crate::spi::touchscreen::init_tsc_dsi();

        crate::init_interrupts();

        let mut buffer: *mut [crate::StorageSector] = &mut [];

        crate::IPC_FIFO_HARDWARE.set_status(1);
        while crate::IPC_FIFO_HARDWARE.read_status() != 1 {}
        crate::IPC_FIFO_HARDWARE.set_status(0);

        crate::MMC_CONTROLLER.tmio_init();

        crate::enable_interrupt(crate::Interrupt::IPCNonEmpty);
        crate::enable_interrupt(crate::Interrupt::VBlank);
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
        let boot_info = &mut (*BOOTINFO_MEM).ntr;
        boot_info.wifi_other[0] = wifi_ver;
        let user = &mut boot_info.firmware_data;
        let mac = &mut boot_info.mac_address;
        SPI_HARDWARE.read_firmware(&mut user.bytes, offset);
        SPI_HARDWARE.read_firmware(mac, 0x36);
        boot_info.wifi_channels = [0x41, 0x10];
        let adcx1 = user.halfwords[0x58 / 2];
        let adcy1 = user.halfwords[0x5A / 2];
        let scrx1 = user.bytes[0x5C];
        let scry1 = user.bytes[0x5D];

        let adcx2 = user.halfwords[0x5E / 2];
        let adcy2 = user.halfwords[0x60 / 2];
        let scrx2 = user.bytes[0x62];
        let scry2 = user.bytes[0x63];

        let mut controller = {
            let x_scale = ((scrx2 as i32 - scrx1 as i32) << 19) / (adcx2 as i32 - adcx1 as i32);
            let y_scale = ((scry2 as i32 - scry1 as i32) << 19) / (adcy2 as i32 - adcy1 as i32);
            let x_offset = (((adcx1 as i32 + adcx2 as i32) * x_scale)
                - ((scrx1 as i32 + scrx2 as i32) << 19))
                / 2;
            let y_offset = (((adcy1 as i32 + adcy2 as i32) * y_scale)
                - ((scry1 as i32 + scry2 as i32) << 19))
                / 2;

            let last_x = 0;
            let last_y = 0;
            let pen_down = false;
            let last_pen = false;
            Controller {
                last_pen,
                pen_down,
                last_x,
                last_y,
                x_scale,
                x_offset,
                y_scale,
                y_offset,
            }
        };
        let mut modcryptor = ModCryptor { console_id };
        loop {
            while IPC_FIFO_HARDWARE.recv_fifo_empty() {}
            let mut response = 0;
            match IPC_FIFO_HARDWARE.recieve_raw_blocking() {
                1 => {
                    assert!(IPC_FIFO_HARDWARE.recieve_value_raw().is_err());
                    response = controller.next_fetch();
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

                5 => {
                    let arg = IPC_FIFO_HARDWARE.recieve_raw_blocking();
                    assert!(IPC_FIFO_HARDWARE.recieve_value_raw().is_err());
                    response = match sd_read_sectors(buffer, arg) {
                        Ok(_) => 0,
                        Err(e) => 0x8000_0000 | e.bits(),
                    }
                }

                6 => {
                    assert!(IPC_FIFO_HARDWARE.recieve_value_raw().is_err());
                    IPC_FIFO_HARDWARE.send_raw_blocking(0);
                    crate::disable_all_interrupts();
                    SOUND_HARDWARE.init();

                    /*
                    AES_HARDWARE.keyslots[0].load_key_x(&[0, 0, 0, 0]);
                    AES_HARDWARE.keyslots[1].load_key_x(&[0, 0, 0, 0]);
                    AES_HARDWARE.keyslots[2].load_key_x(&[0, 0, 0, 0]);
                    AES_HARDWARE.keyslots[3].load_key_x(&[0, 0, 0, 0]);
                    AES_HARDWARE.keyslots[0].load_key_y(&[0, 0, 0, 0]);
                    AES_HARDWARE.keyslots[1].load_key_y(&[0, 0, 0, 0]);
                    AES_HARDWARE.keyslots[2].load_key_y(&[0, 0, 0, 0]);
                    */

                    AES_HARDWARE.init_from_header(
                        &(*(common::bootstrap::BOOTINFO_MEM)).twl_header,
                        console_id,
                    );


                    TIMERS.clear();
                    DMA_HARDWARE.reset();
                    NDMA_HARDWARE.reset();
                    MMC_CONTROLLER.reset();
                    SDIO_CONTROLLER.reset();
                    bootstrap::boot_arm7();
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
                10 => {
                    let arg = IPC_FIFO_HARDWARE.recieve_raw_blocking();
                    assert!(IPC_FIFO_HARDWARE.recieve_value_raw().is_err());
                    response = match write_sd_sectors(arg, buffer) {
                        Ok(_) => 0,
                        Err(e) => e.bits(),
                    }
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

                12 => {
                    assert!(IPC_FIFO_HARDWARE.recieve_value_raw().is_err());
                    response = modcryptor.dewit() as _;
                }
                13 => {
                    let ptr = IPC_FIFO_HARDWARE.recieve_raw_blocking();
                    let len = IPC_FIFO_HARDWARE.recieve_raw_blocking();
                    assert!(IPC_FIFO_HARDWARE.recieve_value_raw().is_err());
                    let firmware = core::slice::from_raw_parts_mut(ptr as *mut u8, len as usize);
                    response = crate::twl_wifi::nwifi_init_complete(wifi_ver, firmware);
                }
                14 => {
                    let ptr = IPC_FIFO_HARDWARE.recieve_raw_blocking();
                    let len = IPC_FIFO_HARDWARE.recieve_raw_blocking();
                    let timer = IPC_FIFO_HARDWARE.recieve_raw_blocking();
                    let flags = IPC_FIFO_HARDWARE.recieve_raw_blocking();
                    let flags = SoundControl::from_bits_retain(flags);
                    let channel = &SOUND_HARDWARE.channels[(len & 0xF) as usize];
                    if flags.contains(SoundControl::START) {
                        channel.length.write(len >> 6);
                        channel.loop_start.write((timer >> 16) as u16);
                        channel.source.write(ptr);
                        channel.timer.write(timer as u16);
                        channel.control.write(flags);
                    } else {
                        channel.control.write(flags);
                    }
                    response = 0;
                }
                15 => {
                    let arg = IPC_FIFO_HARDWARE.recieve_raw_blocking();
                    assert!(IPC_FIFO_HARDWARE.recieve_value_raw().is_err());
                    response = match mmc_write_encrypt(buffer, &key, arg) {
                        Ok(_) => 0,
                        Err(e) => e.bits(),
                    }
                }
                _ => response = 0x8000_0000,
            }
            IPC_FIFO_HARDWARE.send_raw_blocking(response);
        }
    }
}

pub unsafe fn decrypt_module(mem: &mut [u32], mut key: [u32; 4]) {
    AES_HARDWARE.master_control.write(AESCnt::empty());
    AES_HARDWARE.reset();
    AES_HARDWARE.reset();
    AES_HARDWARE.wait_key_busy();
    AES_HARDWARE.set_key_slot(0);
    AES_HARDWARE.wait_key_busy();

    for (d, i) in mem.chunks_exact_mut(4).enumerate() {
        AES_HARDWARE.master_control.write(AESCnt::empty());
        AES_HARDWARE.reset();

        AES_HARDWARE.load_iv(&key);
        add_on_key(&mut key, 1);
        AES_HARDWARE.payload_blocks.write(1);
        AES_HARDWARE.start((0 << 14) | (3 << 12) | (2 << 28) | (1 << 31));

        while AES_HARDWARE.master_control.read().bits() & 0x1F != 0 {}
        for word in i.iter() {
            AES_HARDWARE.write_fifo.write(*word);
        }

        while (AES_HARDWARE.master_control.read().bits() >> 5) & 0x1F != i.len() as u32 {}
        for word in i {
            *word = AES_HARDWARE.read_fifo.read();
        }
    }
    AES_HARDWARE.wait_aes_busy();
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
pub unsafe fn mmc_write_encrypt(
    data: *mut [crate::StorageSector],
    ctr_base: &[u32; 4],
    sector: u32,
) -> Result<(), crate::Status> {
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
    crate::write_sectors(crate::DeviceSelect::EMMC, sector, data)?;
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

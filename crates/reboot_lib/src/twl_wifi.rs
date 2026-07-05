use core::num::NonZeroU32;

use volatile_register::RW;

use crate::{
    ClockCnt, Control, DataControl32, SDIO_CONTROLLER, Status, StorageSector, TMIOPort, i2c::{I2C_HARDWARE, PowerRegister}, ndma::{GlobalControl, NDMAControl}, rtc::RTC_HARDWARE,
};


pub unsafe fn nwifi_init_bmi() -> Result<u32, u32> {

    while SDIO_CONTROLLER.status.read().contains(Status::CMD_BUSY) {}
    PORT.option = 0x40EE;

    
    crate::swi_delay(0xF000);

    

    if sdio_read_func_byte(SDIOFunction::Zero, 0).is_none() {
        PORT.option = 0xC0EE;
    }
    {
        let mut ocr = 0;
        loop {
            while !wifi_card_send_command(crate::mmc::Command::SDIOOpCond, ocr).successful() {}
            ocr = PORT.response[0] & 0xFFFFFF;

            if PORT.response[0] & 0x80000000 > 0 {
                break;
            }
        }
    }
    
    if !wifi_card_send_command(crate::mmc::Command::SetSendRelativeAddr, 0).successful() {
        return Err(1);
    }
    let address = PORT.response[0] & 0xFFFF0000;

    if !wifi_card_send_command(crate::mmc::Command::SelectCard, address).successful() {
        return Err(2);
    }

    if sdio_write_func_byte(SDIOFunction::Zero, 2, 0) {
        return Err(3);
    }

    if sdio_write_func_byte(SDIOFunction::Zero, 2, 2) {
        return Err(4);
    }
    crate::swi_delay(0xF000);

    let Some(mut interface_cnt) = sdio_read_func_byte(SDIOFunction::Zero, 7) else {
        return Err(5)
    };
    interface_cnt |= 0x82;

    if sdio_write_func_byte(SDIOFunction::Zero, 7, interface_cnt) {
        return Err(6);
    }
    
    PORT.option = 0x40EE;


    if sdio_write_func_byte(SDIOFunction::Zero, 0x12, 0x2) {
        return Err(7);
    }

    if sdio_write_func_byte(SDIOFunction::Zero, 0x110, PORT.block_len as u8) {
        return Err(8);
    }
    if sdio_write_func_byte(SDIOFunction::Zero, 0x111, (PORT.block_len >> 8) as u8) {
        return Err(9);
    }

    if sdio_write_func_byte(SDIOFunction::Zero, 0x10, PORT.block_len as u8) {
        return Err(0x10);
    }
    if sdio_write_func_byte(SDIOFunction::Zero, 0x11, (PORT.block_len >> 8) as u8) {
        return Err(0x11);
    }
    crate::swi_delay(0xF000);

    let Some(revision) = sdio_read_func_byte(SDIOFunction::Zero, 0) else {
        return Err(0x12);
    };
    sdio_write_func_byte(SDIOFunction::Zero, 2, 2);
    while sdio_read_func_byte(SDIOFunction::Zero, 3) != Some(2) {}
    let manufacturer = sdio_read_func_word(SDIOFunction::Zero, 0x1007);
    if manufacturer == 0xFFFFFFFF {
        return Err(0x13);
    }
    let Some(chip_id) = nwifi_read_intern_word(0x000040ec) else {return Err(0x14)};

    let interest_addr = if chip_id == 0x02000001 {
        //AR6002
        0x00500400
    } else {
        //AR601x
        0x00520000
    };

    let Some(bmi_ver) = nwifi_get_bmi_version() else {return Err(0x20)};

    if bmi_ver == 0xFFFFFFFF {
        return Err(0x21);
    }


    let Some(is_fw_uploaded) = nwifi_read_intern_word(interest_addr + 0x58) else { return Err(0x16);};

    if is_fw_uploaded == 1 {
        return Ok(0xFFFFFFFF)
    }

    if nwifi_write_intern_word(0x4000, 0x100) {
        return Err(0x17);
    }

    crate::swi_delay(0x10000);

    let Some(reset_cause) = nwifi_read_intern_word(0x40C0) else { return Err(0x18)};

    if reset_cause != 2 {
        return Err(0x19);
    }
    Ok(interest_addr)
}
unsafe fn nwifi_get_bmi_version() ->  Option<u32> {
    wifi_wait_count4();
    if nwifi_write_mbox_u32(8, true) {
        return None;
    }

    let version = nwifi_read_mbox_word()?;
    if version == 0xFFFFFFFF  {
        let len = nwifi_read_mbox_word()?;
        if len != 0xFFFFFFFF {
            let ver = nwifi_read_mbox_word()?;
            for _ in 0..((len/4)-2) {
                nwifi_read_mbox_word();
            }
            Some(ver)
        } else {
            Some(version)
        }
    } else {
        Some(version)
    }
}
unsafe fn nwifi_read_mbox_word() -> Option<u32> {
    while sdio_read_func_byte(SDIOFunction::One, 0x405).map(|i| i & 1 == 0).unwrap_or(true) {}
    let mut val = [0u8; 4];
    for (i, val) in val.iter_mut().enumerate() {
        *val = (sdio_read_func_byte(SDIOFunction::One, 0xFC+i as u32)?);
    }
    Some(u32::from_le_bytes(val))
}


unsafe fn nwifi_write_mbox_u32(mut val: u32, send_irq: bool) -> bool {
    let mut res = false;
    for i in 0..4 {
        let addr = if i == 3 && send_irq { 0xFF } else { 0 };
        res |= sdio_write_func_byte(SDIOFunction::One, addr, val as u8);
        val >>= 8;
    }
    res
}
unsafe fn nwifi_read_intern_word(addr: u32) -> Option<u32> {
    (!sdio_write_func_word(SDIOFunction::One, 0x47C, addr)).then_some(sdio_read_func_word(SDIOFunction::One, 0x474))
}
unsafe fn nwifi_write_intern_word(addr: u32, value: u32) -> bool {
    sdio_write_func_word(SDIOFunction::One, 0x474, value) |
    sdio_write_func_word(SDIOFunction::One, 0x478, addr)
}

static mut PORT: TMIOPort = TMIOPort::dsio();

#[inline(always)]
unsafe fn wifi_card_send_command(command: crate::mmc::Command, arg: u32) -> Status {
    SDIO_CONTROLLER.send_command(&mut PORT, command, arg)
}
pub unsafe fn dsio_hw_init() {
    (*(0x4004008 as *mut RW<u32>)).modify(|i| i | (1<<19) | (1<<23));
     //wifi_card_wlan_init_bmi
    (*(0x4004C04 as *mut RW<u16>)).write(0);
    crate::swi_delay(5 * 134056);

    I2C_HARDWARE.write_register(PowerRegister::WIFILED.into(), 0x13);
    (*(0x4004020 as *mut RW<u16>)).write(1); //SCFG: wifi on?

    SDIO_CONTROLLER.port_select.write(0);
    SDIO_CONTROLLER.data_control_32.write(DataControl32::USE_DATA32 | DataControl32::CLEAR_FIFO_32);
    SDIO_CONTROLLER.data_control.write(Control::USE_DATA32);
    SDIO_CONTROLLER.soft_reset.write(0);
    SDIO_CONTROLLER.soft_reset.write(1);
    SDIO_CONTROLLER.irmask.write(Status::all());
    SDIO_CONTROLLER.ext_card_detect_mask.write(0xDB);
    SDIO_CONTROLLER.ext_card_detect_dat3_mask.write(0xDB);
    SDIO_CONTROLLER.options.write(0x40EE);    
}
pub unsafe fn nwifi_init_complete(wifi_version: u8, firmware: &mut [u8]) -> u32 {
    dsio_hw_init();
    match nwifi_init_bmi() {
        Ok(area) => area,
        Err(err) => return err,
    };
    let firmware = find_firmware_for_card(wifi_version, firmware);
    let Some(interest) = find_interest_addr(firmware) else { return 10};
    let interest = interest.get(); 
    if interest != 0xFFFFFFFF {
        let firmware_upload = upload_wifi_firmware(wifi_version, firmware, interest);
        
        if firmware_upload > 0 {
            return firmware_upload;
        }
    }
    launch_firmware(interest)
}
unsafe fn launch_firmware(interest_area: u32) -> u32 {
    if nwifi_start_firmware() {
        return 0x119
    }
    loop {
        if nwifi_read_intern_word(interest_area+0x58) == Some(1) { break;}
        else {crate::swi::swi_delay(0x100)};
    }
    0
}
fn find_firmware_for_card(version: u8, firmware: &[u8]) -> &[u8] {
    let Some(included_firmwares) = firmware.get(0xa2).copied() else { return &[] };
    let Some(firmware_index) = (0..included_firmwares as usize).into_iter().filter(|i| firmware.get(0xa4+8+(*i*32)).copied() == Some(version)).next() else { return &[]};
    let offset = 0xa4+(firmware_index*32);
    let Some(offset) = firmware.get(offset..).and_then(|i| i.first_chunk::<4>()) else {return &[]};
    let offset = u32::from_le_bytes(offset.clone()) as usize;
    let Some(firmware) = firmware.get(offset..) else { return &[]};
    firmware
}
fn find_interest_addr(firmware: &[u8]) -> Option<NonZeroU32> {
    let first = firmware.first_chunk::<4>()?.clone();
    let id_count = first[1];
    let offset = u16::from_le_bytes([first[2], first[3]]);
    let offset = offset as usize + 4 + (id_count as usize * 8);
    let chunk = firmware.get(offset..).map(|i| i.first_chunk::<4>()).flatten()?;
    NonZeroU32::new(u32::from_le_bytes(chunk.clone()))
}
unsafe fn upload_wifi_firmware(wifi_version: u8, firmware: &[u8], interest_area: u32) -> u32 {
    
    if firmware.is_empty() {
        return 0x104;
    }

    let Ok((part_d, dest)) = get_wifi_part(firmware, FirmwarePart::PartD) else { return 0x105 };
    if wifi_card_upload_binary(dest, part_d) {
        return 0x106
    }

    let Some(bmi_ver) = nwifi_get_bmi_version() else {return 0x106};
    if bmi_ver == 0xFFFFFFFF {
        return 0x106;
    }

    let Ok((part_c, dest)) = get_wifi_part(firmware, FirmwarePart::PartC) else { return 0x107 };
    if wifi_card_upload_binary(dest, part_c) {
        return 0x108
    }

    let Some(bmi_ver) = nwifi_get_bmi_version() else {return 0x108};
    if bmi_ver == 0xFFFFFFFF {
        return 0x108;
    }

    if wifi_card_execute(dest + 0x400000, dest).is_none() {
        return 0x109;
    }
    

    let Ok((part_a, dest)) = get_wifi_part(firmware, FirmwarePart::PartA) else { return 0x110 };
    if wifi_card_upload_binary_lz(dest, part_a) {
        return 0x111
    }

    let Ok((part_d, dest)) = get_wifi_part(firmware, FirmwarePart::PartD) else { return 0x112 };
    if wifi_card_upload_binary(dest, part_d) {
        return 0x113
    }


    let Ok((part_b, dest)) = get_wifi_part(firmware, FirmwarePart::PartB) else { return 0x114 };
    if wifi_card_upload_binary(dest, part_b) {
        return 0x115
    }

    if wifi_card_write_memory(interest_area+0x18, &dest.to_le_bytes()) {
        return 0x116
    }

    if wifi_card_write_memory(interest_area+0x6C, &0x80u32.to_le_bytes()) {
        return 0x117
    }
    if wifi_card_write_memory(interest_area+0x74, &0x63u32.to_le_bytes()) {
        return 0x118
    }
    0
}
unsafe fn nwifi_start_firmware() -> bool {
    wifi_wait_count4();
    nwifi_write_mbox_u32(1, true)
}
#[repr(u8)]
enum FirmwarePart {
    PartA = 0,
    PartB = 1,
    PartC = 2,
    PartD = 3,
}
fn get_wifi_part<'a>(firmware: &'a [u8], part: FirmwarePart) -> Result<(&'a [u8], u32), u32> {
    let parts = &firmware[4..][part as u8 as usize * 16..];
    let Some((offset, rem)) = parts.split_first_chunk::<4>() else { return Err(0x201)};
    let offset = u32::from_le_bytes(offset.clone()) as usize;
    let Some((len, rem)) = rem.split_first_chunk::<4>() else { return Err(0x202)};
    let len = u32::from_le_bytes(len.clone()) as usize;
    let Some((flags, rem)) = rem.split_first_chunk::<4>() else { return Err(0x203)};
    let Some((destination, rem)) = rem.split_first_chunk::<4>() else { return Err(0x204)};
    let destination = u32::from_le_bytes(destination.clone());
    Ok((&firmware[offset..offset+len], destination))
} 
unsafe fn wifi_card_upload_binary(addr: u32, binary: &[u8]) -> bool {
    const CHUNK_SIZE: usize = 0x1F0;
    for (i, chunk) in binary.chunks(CHUNK_SIZE).enumerate() {
        if wifi_card_write_memory(addr + (i*CHUNK_SIZE) as u32, chunk) {
            return true
        }
    }
    false
}
unsafe fn wifi_card_upload_binary_lz(addr: u32, binary: &[u8]) -> bool {
    if wifi_card_start_lz(addr) {
        return true
    }
    //while [0,0xFF].contains(&nwifi_read_func_byte(1, 0x450)) {}
    for chunk in binary.chunks(0x1F8) {
        if wifi_card_upload_lz(chunk) {
            return true;
        }
    }
    false
}

unsafe fn wifi_card_start_lz(addr: u32) -> bool {
    wifi_wait_count4();
    nwifi_write_mbox_u32(0xD, false) |
    nwifi_write_mbox_u32(addr, true)
}
unsafe fn wifi_card_upload_lz(data: &[u8]) -> bool {

    if data.is_empty() {return false}
    let len = (data.len() as u32 + 3) & !3;
    let mut iter = data.iter();

    wifi_wait_count4();
    if nwifi_write_mbox_u32(0xE, false) {
        return true
    }
    if nwifi_write_mbox_u32(len as u32, false) {
        return true
    }

    for _ in 0..(len-1) {
        let byte = iter.next().copied().unwrap_or(0);
        if sdio_write_func_byte(SDIOFunction::One, 0, byte) {
            return true
        }
    }
    let byte = iter.next().copied().unwrap_or(0);
    if sdio_write_func_byte(SDIOFunction::One, 0xFF, byte) {
        return true
    }

    false
}
unsafe fn wifi_wait_count4() {
    while sdio_read_func_byte(SDIOFunction::One, 0x450).map(|i| i == 0).unwrap_or(true) {
        crate::swi::swi_delay(0x100);
    }
}
unsafe fn wifi_card_write_memory(addr: u32, data: &[u8]) -> bool {
    if data.is_empty() {return false}
    let len = (data.len() as u32 + 3) & !3;
    let mut iter = data.iter();
    wifi_wait_count4();
    if nwifi_write_mbox_u32(3, false) {
        return true
    }
    if nwifi_write_mbox_u32(addr, false) {
        return true
    }
    if nwifi_write_mbox_u32(len, false) {
        return true
    }

    for _ in 0..(len-1) {
        let byte = iter.next().copied().unwrap_or(0);
        if sdio_write_func_byte(SDIOFunction::One, 0, byte) {
            return true
        }
    }
    let byte = iter.next().copied().unwrap_or(0);
    if sdio_write_func_byte(SDIOFunction::One, 0xFF, byte) {
        return true
    }
    false
}
unsafe fn wifi_card_execute(addr: u32, arg: u32) -> Option<u32> {
    wifi_wait_count4();
    nwifi_write_mbox_u32(4, false);
    nwifi_write_mbox_u32(addr, false);
    nwifi_write_mbox_u32(arg, true);
    nwifi_read_mbox_word()
}
#[repr(u32)]
#[derive(Clone, Copy)]
enum SDIOFunction {
    Zero = (0<<28),
    One = (1<<28),
}
unsafe fn sdio_read_func_byte(func: SDIOFunction, addr: u32) -> Option<u8> {
    let arg = func as u32 | ((addr & 0x1FFFF) << 9);
    wifi_card_send_command(crate::mmc::Command::SDIORegRW, arg).successful().then_some(PORT.response[0] as u8)
}
unsafe fn sdio_read_func_halfword(func: SDIOFunction, addr: u32) -> u16 {
    sdio_read_func_byte(func, addr).unwrap_or(0xFF) as u16 | ((sdio_read_func_byte(func, addr+1).unwrap_or(0xFF) as u16) << 8)
}
unsafe fn sdio_read_func_word(func: SDIOFunction, addr: u32) -> u32 {
    sdio_read_func_halfword(func, addr) as u32 | ((sdio_read_func_halfword(func, addr+2) as u32) << 16)
}
unsafe fn sdio_write_func_byte(func: SDIOFunction, addr: u32, value: u8) -> bool {
    let arg = func as u32 | ((addr & 0x1FFFF) << 9) | (1<<31) | (value as u32);
    !wifi_card_send_command(crate::mmc::Command::SDIORegRW, arg).successful()
}
unsafe fn sdio_write_func_halfword(func: SDIOFunction, addr: u32, value: u16) -> bool {
    sdio_write_func_byte(func, addr+1, (value >> 8) as u8)
    | sdio_write_func_byte(func, addr,value as u8)
    
}

unsafe fn sdio_write_func_word(func: SDIOFunction, addr: u32, value: u32) -> bool {
    sdio_write_func_halfword(func, addr+2, (value >> 16) as u16)
    | sdio_write_func_halfword(func, addr,value as u16)
    
}
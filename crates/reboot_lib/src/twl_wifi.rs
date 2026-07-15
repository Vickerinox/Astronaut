use core::{num::NonZeroU32, ptr::addr_of_mut};

use volatile_register::RW;

use crate::{
    i2c::{PowerRegister, I2C_HARDWARE},
    ndma::{GlobalControl, NDMAControl},
    rtc::RTC_HARDWARE,
    ClockCnt, Control, DataControl32, Status, StorageSector, TMIOPort, SDIO_CONTROLLER,
};

unsafe fn nwifi_restart_card() -> bool {
    PORT.option = 0x40EE;
    SDIO_CONTROLLER.wait_busy();
    #[allow(static_mut_refs)]
    SDIO_CONTROLLER.prepare_port(&mut PORT);
    if sdio_read_func_byte(SDIOFunc::Zero, 0).is_none() {
        PORT.option = 0xC0EE;
    }
    {
        let mut ocr = 0;
        loop {
            while !wifi_card_send_command(crate::mmc::Command::SDIOOpCond, ocr, true).successful() {
            }
            ocr = PORT.response[0] & (1 << 20);

            if PORT.response[0] & 0x80000000 > 0 {
                break;
            }
        }
    }

    if !wifi_card_send_command(crate::mmc::Command::SetSendRelativeAddr, 0, false).successful() {
        return true;
    }
    let address = PORT.response[0] & 0xFFFF0000;

    if !wifi_card_send_command(crate::mmc::Command::SelectCard, address, false).successful() {
        return true;
    }

    if sdio_write_func_byte(SDIOFunc::Zero, 2, 0) {
        return true;
    }

    if sdio_write_func_byte(SDIOFunc::Zero, 2, 2) {
        return true;
    }
    crate::swi_delay(0xF000);

    let Some(mut interface_cnt) = sdio_read_func_byte(SDIOFunc::Zero, 7) else {
        return true;
    };
    interface_cnt |= 0x82;

    if sdio_write_func_byte(SDIOFunc::Zero, 7, interface_cnt) {
        return true;
    }

    PORT.option = 0x40EE;
    SDIO_CONTROLLER.wait_busy();
    #[allow(static_mut_refs)]
    SDIO_CONTROLLER.prepare_port(&mut PORT);
    false
}
unsafe fn nwifi_init_func0() -> bool {
    if sdio_write_func_byte(SDIOFunc::Zero, 0x12, 0x2) {
        return true;
    }

    if sdio_write_func_byte(SDIOFunc::Zero, 0x110, PORT.block_len as u8) {
        return true;
    }
    if sdio_write_func_byte(SDIOFunc::Zero, 0x111, (PORT.block_len >> 8) as u8) {
        return true;
    }

    if sdio_write_func_byte(SDIOFunc::Zero, 0x10, PORT.block_len as u8) {
        return true;
    }
    if sdio_write_func_byte(SDIOFunc::Zero, 0x11, (PORT.block_len >> 8) as u8) {
        return true;
    }
    crate::swi_delay(0xF000);

    while sdio_read_func_byte(SDIOFunc::Zero, 3) != Some(2) {}
    false
}
unsafe fn nwifi_init_func1() -> bool {
    /*
    let Some(is_fw_uploaded) = nwifi_read_intern_word(interest_addr + 0x58) else {
        return Err(0x16);
    };

    if is_fw_uploaded == 1 {
        return Ok(0xFFFFFFFF);
    }
     */

    if nwifi_write_intern_word(0x4000, 0x100) {
        return true;
    }

    crate::swi_delay(0x10000);

    if nwifi_read_intern_word(0x40C0) != Some(2) {
        return true;
    }
    false
}
unsafe fn nwifi_read_mbox_word() -> Option<u32> {
    while sdio_read_func_byte(SDIOFunc::One, 0x405)
        .map(|i| i & 1 == 0)
        .unwrap_or(true)
    {}
    let mut val = [0u8; 4];
    for (i, val) in val.iter_mut().enumerate() {
        *val = (sdio_read_func_byte(SDIOFunc::One, 0xFC + i as u32)?);
    }
    Some(u32::from_le_bytes(val))
}

unsafe fn nwifi_write_mbox_u32(mut val: u32, send_irq: bool) -> bool {
    let mut res = false;
    let val = val.to_le_bytes();
    let addr = if send_irq { 0xFC } else { 0 };
    for i in 0..4 {
        res |= sdio_write_func_byte(SDIOFunc::One, addr + i, val[i as usize]);
    }
    res
}
unsafe fn nwifi_read_intern_word(addr: u32) -> Option<u32> {
    (!sdio_write_func_word(SDIOFunc::One, 0x47C, addr))
        .then_some(sdio_read_func_word(SDIOFunc::One, 0x474))
}
unsafe fn nwifi_write_intern_word(addr: u32, value: u32) -> bool {
    sdio_write_func_word(SDIOFunc::One, 0x474, value)
        | sdio_write_func_word(SDIOFunc::One, 0x478, addr)
}

static mut PORT: TMIOPort = TMIOPort::dsio();

#[inline(always)]
#[allow(static_mut_refs)]
unsafe fn wifi_card_send_command(
    command: crate::mmc::Command,
    arg: u32,
    switch_port: bool,
) -> Status {
    SDIO_CONTROLLER.wait_busy();
    if switch_port {
        SDIO_CONTROLLER.prepare_port(&mut PORT);
    }
    SDIO_CONTROLLER.send_command_inner(&mut PORT, command, arg)
}
pub unsafe fn dsio_hw_init() {
    (*(0x4004008 as *mut RW<u32>)).modify(|i| i | (1 << 19) | (1 << 23));
    //wifi_card_wlan_init_bmi
    (*(0x4004C04 as *mut RW<u16>)).write(0);
    crate::swi_delay(5 * 134056);

    I2C_HARDWARE.write_register(PowerRegister::WIFILED.into(), 0x13);
    (*(0x4004020 as *mut RW<u16>)).write(1); //SCFG: wifi on?

    SDIO_CONTROLLER.port_select.write(0);
    SDIO_CONTROLLER
        .data_control_32
        .write(DataControl32::USE_DATA32 | DataControl32::CLEAR_FIFO_32);
    SDIO_CONTROLLER.data_control.write(Control::USE_DATA32);
    SDIO_CONTROLLER.soft_reset.write(0);
    SDIO_CONTROLLER.soft_reset.write(1);
    SDIO_CONTROLLER.irmask.write(Status::all());
    SDIO_CONTROLLER.ext_card_detect_mask.write(0xDB);
    SDIO_CONTROLLER.ext_card_detect_dat3_mask.write(0xDB);
    SDIO_CONTROLLER.options.write(0x40EE);
}
pub unsafe fn nwifi_init_complete(wifi_version: u8, firmware: &mut [u8]) -> u32 {
    // TODO: find a better way to detect already uploaded firmware?
    if I2C_HARDWARE
        .read_register(PowerRegister::WIFILED.into())
        .ok()
        == Some(0x13)
    {
        return 0;
    }
    //set initial registers
    dsio_hw_init();
    //perform reset on the card (enough to be sure were in 4-bit bus width)
    if nwifi_restart_card() {
        return 1;
    }
    //perform func0 initialization (which readies us for func1)
    if nwifi_init_func0() {
        return 2;
    }
    //perform func1 initialization
    if nwifi_init_func1() {
        return 3;
    }

    //find out what the base address of the data segment for this card is
    let Some(interest_area) = find_interest_addr(firmware) else {
        return 5;
    };

    let data_base = interest_area.get();

    // Upload segment D (bootstub data)
    let Some((part_d, dest)) = get_wifi_part(firmware, FirmwarePart::PartD) else {
        return 6;
    };
    if wifi_card_upload_binary(dest, part_d) {
        return 6;
    }
    // Upload segment C (bootstub code)
    let Some((part_c, dest)) = get_wifi_part(firmware, FirmwarePart::PartC) else {
        return 7;
    };
    if wifi_card_upload_binary(dest, part_c) {
        return 7;
    }
    // Execute bootstub code
    if wifi_card_execute(dest + 0x400000, dest).is_none() {
        return 8;
    }
    // Upload segment A (firmware code)
    let Some((part_a, dest)) = get_wifi_part(firmware, FirmwarePart::PartA) else {
        return 9;
    };
    if wifi_card_upload_binary_lz(dest, part_a) {
        return 9;
    }
    // Upload segment B (firmware data, reffered to as "firmware database" by nocash)
    let Some((part_b, dest)) = get_wifi_part(firmware, FirmwarePart::PartB) else {
        return 10;
    };
    if wifi_card_upload_binary(dest, part_b) {
        return 10;
    }
    // Write these random words which i have no idea what they do
    if wifi_card_write_memory(data_base + 0x18, &mut dest.to_le_bytes()) {
        return 11;
    }
    if wifi_card_write_memory(data_base + 0x6C, &mut 0x80u32.to_le_bytes()) {
        return 11;
    }
    if wifi_card_write_memory(data_base + 0x74, &mut 0x63u32.to_le_bytes()) {
        return 11;
    }
    // Start the firmware
    if nwifi_start_firmware() {
        return 12;
    }
    // Get confirmation that the firmware is launched
    loop {
        if nwifi_read_intern_word(data_base + 0x58) == Some(1) {
            break;
        } else {
            crate::swi::swi_delay(0x100)
        };
    }
    0
}

fn find_firmware_for_card(version: u8, firmware: &mut [u8]) -> Option<&mut [u8]> {
    let firmware_count = firmware.get(0xa2).copied()?;
    let firmware_index = (0..firmware_count as usize)
        .into_iter()
        .filter(|i| firmware.get(0xa4 + 8 + (*i * 32)).copied() == Some(version))
        .next()?;
    let offset = {
        let firmware_offset = 0xa4 + (firmware_index * 32);
        let offset = firmware
            .get(firmware_offset..)
            .and_then(|i| i.first_chunk::<4>())?;
        u32::from_le_bytes(offset.clone()) as usize
    };
    let firmware = firmware.get_mut(offset..)?;
    Some(firmware)
}
fn find_interest_addr(firmware: &[u8]) -> Option<NonZeroU32> {
    let first = firmware.first_chunk::<4>()?.clone();
    let id_count = first[1];
    let offset = u16::from_le_bytes([first[2], first[3]]);
    let offset = offset as usize + 4 + (id_count as usize * 8);
    let chunk = firmware
        .get(offset..)
        .map(|i| i.first_chunk::<4>())
        .flatten()?;
    NonZeroU32::new(u32::from_le_bytes(chunk.clone()))
}
unsafe fn upload_wifi_firmware(wifi_version: u8, firmware: &[u8], interest_area: u32) -> u32 {
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
fn get_wifi_part<'a>(firmware: &'a mut [u8], part: FirmwarePart) -> Option<(&'a mut [u8], u32)> {
    let parts = firmware.get(4 + (part as u8 as usize * 16)..)?;
    let (offset, rem) = parts.split_first_chunk::<4>()?;
    let offset = u32::from_le_bytes(offset.clone()) as usize;
    let (len, rem) = rem.split_first_chunk::<4>()?;
    let len = u32::from_le_bytes(len.clone()) as usize;
    let (flags, rem) = rem.split_first_chunk::<4>()?;
    let (destination, rem) = rem.split_first_chunk::<4>()?;
    let destination = u32::from_le_bytes(destination.clone());
    firmware
        .get_mut(offset..offset + len)
        .map(|i| (i, destination))
}
unsafe fn wifi_card_upload_binary(addr: u32, binary: &mut [u8]) -> bool {
    const CHUNK_SIZE: usize = 0x1F0;
    for (i, chunk) in binary.chunks_mut(CHUNK_SIZE).enumerate() {
        if wifi_card_write_memory(addr + (i * CHUNK_SIZE) as u32, chunk) {
            return true;
        }
    }
    false
}
unsafe fn wifi_card_upload_binary_lz(addr: u32, binary: &mut [u8]) -> bool {
    if wifi_card_start_lz(addr) {
        return true;
    }
    for chunk in binary.chunks_mut(0x1F8) {
        if wifi_card_upload_lz(chunk) {
            return true;
        }
    }
    false
}

unsafe fn wifi_card_start_lz(addr: u32) -> bool {
    wifi_wait_count4();
    nwifi_write_mbox_u32(0xD, false) | nwifi_write_mbox_u32(addr, true)
}
unsafe fn wifi_card_upload_lz(data: &mut [u8]) -> bool {
    if data.is_empty() {
        return false;
    }

    let total_len = (data.len() as u32 + 3) & !3;
    wifi_wait_count4();
    if nwifi_write_mbox_u32(0xE, false) {
        return true;
    }

    if nwifi_write_mbox_u32(total_len as u32, false) {
        return true;
    }

    let (blocks, snipit_data) = data.split_at_mut(data.len() & !0x7f);
    if !blocks.is_empty() {
        let mut buf: &mut [u32] = bytemuck::cast_slice_mut(blocks);
        PORT.buffer = buf;

        let addr = 0x1000 - total_len as u32;
        let res = SDIO_CONTROLLER.send_command(
            &mut PORT,
            crate::mmc::Command::SDIORegWBlock,
            ((1 << 31)
                | (1 << 28)
                | (1 << 27)
                | (1 << 26)
                | ((addr & 0x1FFFF) << 9)
                | ((buf.len() / 0x20) as u32)),
        );
        if !res.successful() {
            return true;
        }
    }

    let snipit_len = (snipit_data.len() as u32 + 3) & !3;
    let mut iter = snipit_data.iter();

    for _ in 0..(snipit_len - 1) {
        let byte = iter.next().copied().unwrap_or(0);
        if sdio_write_func_byte(SDIOFunc::One, 0, byte) {
            return true;
        }
    }
    let byte = iter.next().copied().unwrap_or(0);
    if sdio_write_func_byte(SDIOFunc::One, 0xFF, byte) {
        return true;
    }

    false
}
unsafe fn wifi_wait_count4() {
    while sdio_read_func_byte(SDIOFunc::One, 0x450)
        .map(|i| i == 0)
        .unwrap_or(true)
    {
        crate::swi::swi_delay(0x100);
    }
}
unsafe fn wifi_card_write_memory(addr: u32, data: &mut [u8]) -> bool {
    if data.is_empty() {
        return false;
    }

    let total_len = (data.len() as u32 + 3) & !3;
    wifi_wait_count4();
    if nwifi_write_mbox_u32(3, false) {
        return true;
    }
    if nwifi_write_mbox_u32(addr, false) {
        return true;
    }
    if nwifi_write_mbox_u32(total_len, false) {
        return true;
    }

    let (blocks, snipit_data) = data.split_at_mut(data.len() & !0x7f);
    if !blocks.is_empty() {
        let mut buf: &mut [u32] = bytemuck::cast_slice_mut(blocks);
        PORT.buffer = buf;

        let addr = 0x1000 - total_len as u32;
        let res = SDIO_CONTROLLER.send_command(
            &mut PORT,
            crate::mmc::Command::SDIORegWBlock,
            ((1 << 31)
                | (1 << 28)
                | (1 << 27)
                | (1 << 26)
                | ((addr & 0x1FFFF) << 9)
                | ((buf.len() / 0x20) as u32)),
        );
        if !res.successful() {
            return true;
        }
    }

    let len = (snipit_data.len() as u32 + 3) & !3;
    let mut iter = snipit_data.iter();

    for _ in 0..(len - 1) {
        let byte = iter.next().copied().unwrap_or(0);
        if sdio_write_func_byte(SDIOFunc::One, 0, byte) {
            return true;
        }
    }
    let byte = iter.next().copied().unwrap_or(0);
    if sdio_write_func_byte(SDIOFunc::One, 0xFF, byte) {
        return true;
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
enum SDIOFunc {
    Zero = (0 << 28),
    One = (1 << 28),
}
unsafe fn sdio_read_func_byte(func: SDIOFunc, addr: u32) -> Option<u8> {
    let arg = func as u32 | ((addr & 0x1FFFF) << 9);
    wifi_card_send_command(crate::mmc::Command::SDIORegRW, arg, false)
        .successful()
        .then_some(PORT.response[0] as u8)
}
unsafe fn sdio_read_func_halfword(func: SDIOFunc, addr: u32) -> u16 {
    sdio_read_func_byte(func, addr).unwrap_or(0xFF) as u16
        | ((sdio_read_func_byte(func, addr + 1).unwrap_or(0xFF) as u16) << 8)
}
unsafe fn sdio_read_func_word(func: SDIOFunc, addr: u32) -> u32 {
    sdio_read_func_halfword(func, addr) as u32
        | ((sdio_read_func_halfword(func, addr + 2) as u32) << 16)
}
unsafe fn sdio_write_func_byte(func: SDIOFunc, addr: u32, value: u8) -> bool {
    let arg = func as u32 | ((addr & 0x1FFFF) << 9) | (1 << 31) | (value as u32);
    !wifi_card_send_command(crate::mmc::Command::SDIORegRW, arg, false).successful()
}
unsafe fn sdio_write_func_halfword(func: SDIOFunc, addr: u32, value: u16) -> bool {
    sdio_write_func_byte(func, addr + 1, (value >> 8) as u8)
        | sdio_write_func_byte(func, addr, value as u8)
}

unsafe fn sdio_write_func_word(func: SDIOFunc, addr: u32, value: u32) -> bool {
    sdio_write_func_halfword(func, addr + 2, (value >> 16) as u16)
        | sdio_write_func_halfword(func, addr, value as u16)
}

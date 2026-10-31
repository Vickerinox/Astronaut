use alloc::string::String;
use reboot_lib::{bytemuck, fatfs_embedded::{self, fatfs::FileOptions}};

use crate::boot::read_all;

unsafe impl bytemuck::Zeroable for WiFiFirmwareFileHeader {}
unsafe impl bytemuck::AnyBitPattern for WiFiFirmwareFileHeader {}

unsafe impl bytemuck::Zeroable for WifiFirmwareEntry {}
unsafe impl bytemuck::AnyBitPattern for WifiFirmwareEntry {}

unsafe impl bytemuck::Zeroable for AssumedHeader {}
unsafe impl bytemuck::AnyBitPattern for AssumedHeader {}
unsafe impl bytemuck::NoUninit for AssumedHeader {}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct AssumedHeader {
    header: WiFiFirmwareFileHeader,
    entries: [WifiFirmwareEntry; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct WiFiFirmwareFileHeader {
    version_number: u16,
    firmwares_included: u8,
    _reserved: u8,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct WifiFirmwareEntry {
    offset: u32,
    size: u32,
    wifi_board_type: u32,
    sha1_hash: [u8; 0x14],
}



unsafe fn find_wifi_firmware_path() -> Option<String> {
    const CONTENT_FOLDER: &str = "nand:/title/0003000F/484E4341/content/";
    let app_version = {
        let mut firmware_tmd = fatfs_embedded::open(
            &mut alloc::format!("{CONTENT_FOLDER}title.tmd"),
            FileOptions::Read,
        )
        .ok()?;
        let mut app_version = [0u8; 4];
        fatfs_embedded::seek(&mut firmware_tmd, 0x1E4).ok()?;
        read_all(&mut app_version, &mut firmware_tmd).ok()?;
        u32::from_be_bytes(app_version)
    };
    Some(alloc::format!("{CONTENT_FOLDER}{app_version:08x?}.app"))
}
fn find_firmware_for_card(header: &AssumedHeader, version: u8) -> Option<WifiFirmwareEntry> {
    header.entries
        .get(..header.header.firmwares_included as usize)?
        .into_iter()
        .filter(|i| i.wifi_board_type == version as u32)
        .next().cloned()
}
unsafe fn get_wifi_firmware(
    wifi_ver: u8,
) -> Option<(fatfs_embedded::fatfs::File, alloc::alloc::Layout)> {
    let mut firmware_path = find_wifi_firmware_path()?;
    let mut firmware = fatfs_embedded::open(&mut firmware_path, FileOptions::Read).ok()?;
    // Seek to end of signature "preheader"
    fatfs_embedded::seek(&mut firmware, 0xA0).ok()?;
    // Read and parse "true header"
    let mut header = AssumedHeader::default();
    read_all(bytemuck::bytes_of_mut(&mut header), &mut firmware).ok()?;
    let WifiFirmwareEntry { offset, size, .. } = find_firmware_for_card(&header, wifi_ver)?;

    // Seek to firmware and provide a memory layout required to store it
    fatfs_embedded::seek(&mut firmware, offset).ok()?;
    let layout = core::alloc::Layout::from_size_align(size as usize, 4).ok()?;
    Some((firmware, layout))
}
pub unsafe fn load_wifi_firmware(wifi_ver: u8) -> u32 {
    let mut ret = 100;
    let Some((mut firmware, layout)) = get_wifi_firmware(wifi_ver) else {
        return ret;
    };
    let firmware_ptr = alloc::alloc::alloc(layout);
    let mut firmware_buffer = core::slice::from_raw_parts_mut(firmware_ptr, layout.size());

    if read_all(&mut firmware_buffer, &mut firmware).is_ok() {
        ret = match reboot_lib::arm9_init_nwifi(firmware_buffer) {
            Ok(_) => 0,
            Err(e) => e.get(),
        };
    }

    alloc::alloc::dealloc(firmware_ptr, layout);
    ret
}
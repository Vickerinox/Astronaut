use core::ops::BitOr;

use crate::bootstrap::{BOOTINFO_MEM, HeaderTWL};

pub unsafe fn init(header: &HeaderTWL, pub_sav_path: &str, prv_sav_path: &str, _location: &str) {
    if header.arm7_device_list == 0 {
        return;
    }
    let list = &mut (*BOOTINFO_MEM).device_list_copy;
    list.clear();
    let mut next_entry = 0;

    list.drives[next_entry] = DeviceEntry::new(b'I', DeviceFlags::SDMC_SLOT, DeviceRights::READ_WRITE, b"sdmc", b"/");
    next_entry += 1;

    list.drives[next_entry] = DeviceEntry::new(b'A', DeviceFlags::TWL_MAIN, DeviceRights::NONE, b"nand", b"/");
    next_entry += 1;

    list.drives[next_entry] = DeviceEntry::new(b'B', DeviceFlags::TWL_PHOTO, DeviceRights::NONE, b"nand2", b"/");
    next_entry += 1;

    list.drives[next_entry] = DeviceEntry::new(b'D', DeviceFlags::FOLDERBASED | DeviceFlags::NAND | DeviceFlags::PARTITION_ONE, DeviceRights::WRITE, b"shared1", b"nand:/shared1");
    next_entry += 1;

    list.drives[next_entry] = DeviceEntry::new(b'F', DeviceFlags::FOLDERBASED | DeviceFlags::NAND | DeviceFlags::PARTITION_TWO, DeviceRights::READ_WRITE, b"photo", b"nand2:/photo");
    next_entry += 1;

    if header.private_save_size != 0 && !prv_sav_path.is_empty() {
        list.drives[next_entry] = DeviceEntry::new(b'G', DeviceFlags::FILEBASED, DeviceRights::READ_WRITE, b"dataPrv", b"sdmc:/LAUNCH~1.NDS");
        next_entry += 1;
    }
    if header.public_save_size != 0 && !pub_sav_path.is_empty() {
        list.drives[next_entry] = DeviceEntry::new(b'H', DeviceFlags::FILEBASED, DeviceRights::READ_WRITE, b"dataPub", b"nand2:/photo");
        //next_entry += 1;
    }

    let path = b"sdmc:/photod.nds"; //location.as_bytes();

    list.app_path[..path.len()].copy_from_slice(path);

}


#[repr(C)]
#[derive(Clone)]
pub struct DeviceList {
    drives: [DeviceEntry; 11],
    _0x39c: [u8; 0x24],
    app_path: [u8; 64],
}
impl DeviceList {
    pub fn clear(&mut self) {
        self.drives = [DeviceEntry::EMPTY; 11];
        self.app_path = [0; 64];
    }
}
pub struct DeviceFlags(u8);
impl DeviceFlags {
    pub const PHYSICAL: Self = Self(0<<3);
    pub const FILEBASED: Self = Self(1<<3);
    pub const FOLDERBASED: Self = Self(2<<3);

    pub const SDMC: Self = Self(0);
    pub const NAND: Self = Self(1);
    
    pub const PARTITION_ONE: Self = Self(0<<5);
    pub const PARTITION_TWO: Self = Self(1<<5);

    pub const ENCRYPTED: Self = Self(1<<7);

    pub const SDMC_SLOT: Self = Self(Self::PHYSICAL.0 | Self::SDMC.0 | Self::PARTITION_ONE.0);
    pub const TWL_MAIN: Self = Self(Self::PHYSICAL.0 | Self::NAND.0 | Self::PARTITION_ONE.0 | Self::ENCRYPTED.0);
    pub const TWL_PHOTO: Self = Self(Self::PHYSICAL.0 | Self::NAND.0 | Self::PARTITION_TWO.0 | Self::ENCRYPTED.0);
}
impl BitOr for DeviceFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}
pub struct DeviceRights(u8);
impl DeviceRights {
    pub const READ: Self = Self(1<<1);
    pub const WRITE: Self = Self(1<<2);
    pub const READ_WRITE: Self = Self(6);
    pub const NONE: Self = Self(0);
}
#[repr(C)]
#[derive(Clone)]
pub struct DeviceEntry {
    drive_letter: u8,
    drive_flags: u8,
    access_rights: u8,
    _0x3: u8,
    device_name: [u8; 16],
    device_path: [u8; 64],
}
impl DeviceEntry {
    pub const EMPTY: Self = Self {
        drive_letter: 0,
        drive_flags: 0, 
        access_rights: 0, 
        _0x3: 0, 
        device_name: [0; _],
        device_path: [0; _], 
    };
    pub fn new(drive_letter: u8, drive_flags: DeviceFlags, access_rights: DeviceRights, name: &[u8], path: &[u8]) -> Self {
        let mut device_name = [0u8; 16];
        device_name[..name.len()].copy_from_slice(name);

        let mut device_path = [0u8; 64];
        device_path[..path.len()].copy_from_slice(path);
        Self { drive_letter, drive_flags: drive_flags.0, access_rights: access_rights.0, _0x3: 0, device_name, device_path }
    }
}
use core::ops::BitOr;

use crate::bootstrap::{BootInfoTWL, TWLHeader, BOOTINFO_MEM};
pub struct DeviceListBuilder<'a> {
    list: &'a mut DeviceList,
    drive_count: usize,
    app_path: &'a str,
}
impl<'a> DeviceListBuilder<'a> {
    pub fn new(list: &'a mut DeviceList, app_path: &'a str) -> Self {
        list.clear();
        list.app_path[..app_path.len()].copy_from_slice(app_path.as_bytes());
        Self {
            list,
            drive_count: 0,
            app_path,
        }
    }
    pub fn add_drive(&mut self, drive: DeviceEntry) -> &mut Self {
        self.list.drives[self.drive_count] = drive;
        self.drive_count += 1;
        self
    }
}
pub fn init(header: &mut BootInfoTWL, app_path: &str, pub_sav_path: &str, prv_sav_path: &str) {
    if header.twl_header.arm7_device_list == 0 {
        return;
    }
    let list = &mut header.device_list_copy;
    let mut list_builder = DeviceListBuilder::new(list, app_path);

    let nand_properties = match app_path.get(..4) {
        Some("nand") => DeviceFlags::COMBO_TWL_MAIN,
        Some("sdmc") => DeviceFlags::COMBO_SDMC_SLOT,
        _ => DeviceFlags::COMBO_TWL_MAIN,
    };

    list_builder
        .add_drive(DeviceEntry::new(
            b'I',
            DeviceFlags::COMBO_SDMC_SLOT,
            DeviceRights::READ_WRITE,
            "sdmc",
            "/",
        ))
        .add_drive(DeviceEntry::new(
            b'A',
            nand_properties,
            DeviceRights::NONE,
            "nand",
            "/",
        ))
        .add_drive(DeviceEntry::new(
            b'B',
            DeviceFlags::COMBO_TWL_PHOTO,
            DeviceRights::NONE,
            "nand2",
            "/",
        ))
        .add_drive(DeviceEntry::new(
            b'D',
            DeviceFlags::FOLDERBASED | DeviceFlags::DRIVE_NAND | DeviceFlags::PARTITION_ONE,
            DeviceRights::WRITE,
            "shared1",
            "nand:/shared1",
        ))
        .add_drive(DeviceEntry::new(
            b'F',
            DeviceFlags::FOLDERBASED | DeviceFlags::DRIVE_NAND | DeviceFlags::PARTITION_TWO,
            DeviceRights::READ_WRITE,
            "photo",
            "nand2:/photo",
        ));
    if header.twl_header.private_save_size != 0 {
        add_save(&mut list_builder, prv_sav_path, "dataPrv", b'G');
    }
    if header.twl_header.public_save_size != 0 {
        add_save(&mut list_builder, pub_sav_path, "dataPub", b'H');
    }
    /*
    list_builder.add_drive(DeviceEntry::new(
        b'C',
        DeviceFlags::FILEBASED | DeviceFlags::DRIVE_NAND,
        DeviceRights::READ_WRITE,
        "share",
        "nand:/shared2/0000",
    ));
    */
}
pub fn add_save(builder: &mut DeviceListBuilder, path: &str, name: &str, drive: u8) {
    let drive_sort = DeviceFlags::FILEBASED;
    builder.add_drive(DeviceEntry::new(
        drive,
        drive_sort,
        DeviceRights::READ_WRITE,
        name,
        path,
    ));
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
    pub const PHYSICAL: Self = Self(0 << 3);
    pub const FILEBASED: Self = Self(1 << 3);
    pub const FOLDERBASED: Self = Self(2 << 3);

    pub const DRIVE_SDMC: Self = Self(0);
    pub const DRIVE_NAND: Self = Self(1);

    pub const PARTITION_ONE: Self = Self(0 << 5);
    pub const PARTITION_TWO: Self = Self(1 << 5);

    pub const ENCRYPTED: Self = Self(1 << 7);

    pub const COMBO_SDMC_SLOT: Self =
        Self(Self::PHYSICAL.0 | Self::DRIVE_SDMC.0 | Self::PARTITION_ONE.0);
    pub const COMBO_TWL_MAIN: Self =
        Self(Self::PHYSICAL.0 | Self::DRIVE_NAND.0 | Self::PARTITION_ONE.0 | Self::ENCRYPTED.0);
    pub const COMBO_TWL_PHOTO: Self =
        Self(Self::PHYSICAL.0 | Self::DRIVE_NAND.0 | Self::PARTITION_TWO.0 | Self::ENCRYPTED.0);
}
impl BitOr for DeviceFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}
pub struct DeviceRights(u8);
impl DeviceRights {
    pub const READ: Self = Self(1 << 1);
    pub const WRITE: Self = Self(1 << 2);
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
    pub fn new(
        drive_letter: u8,
        drive_flags: DeviceFlags,
        access_rights: DeviceRights,
        name: &str,
        path: &str,
    ) -> Self {
        let mut device_name = [0u8; 16];
        device_name[..name.len()].copy_from_slice(name.as_bytes());

        let mut device_path = [0u8; 64];
        device_path[..path.len()].copy_from_slice(path.as_bytes());
        Self {
            drive_letter,
            drive_flags: drive_flags.0,
            access_rights: access_rights.0,
            _0x3: 0,
            device_name,
            device_path,
        }
    }
}

reboot_lib::bitflags::bitflags! {
    #[derive(Clone, Copy)]
    pub struct StatusFlags: u8 {
        const DIRTY = 1;
        const IO_ERROR = 2;
    }
}

pub struct JumpInstruction([u8; 3]);
pub struct SectorSignature([u8; 2]);
impl SectorSignature {
    pub fn valid(&self) -> bool {
        self.0 == [0x55, 0xAA]
    }
}
#[repr(C)]
pub struct BootSector {
    jump_instruction: JumpInstruction,
    oem_name: [u8; 8],
    bios_parameters: BiosParameters,
    _0x5a: [u8; 419], //uninteresting boot code
    drive_number: u8,
    signature: SectorSignature,
}
impl BootSector {
    pub fn evaluate(&self) -> Result<FSType, BadFSError> {
        if !self.signature.valid() {
            return Err(BadFSError::InvalidBootSign);
        }
        self.bios_parameters.evaluate()
    }
    //EWWWWW big endian shit
    pub fn flip_endians(&mut self) {
        let BiosParameters {
            bytes_per_sector,
            reserved_sectors,
            root_entries,
            total_sectors_16,
            media_type,
            sectors_per_fat_16,
            sectors_per_track,
            heads,
            hidden_sectors,
            total_sectors_32,
            sectors_per_fat_32,
            extended_flags,
            fs_version,
            root_dir_first_cluster,
            fs_info_sector,
            backup_boot_sector,
            volume_id,
            ..
        } = self.bios_parameters.clone();
        self.bios_parameters.bytes_per_sector = bytes_per_sector.swap_bytes();
        self.bios_parameters.reserved_sectors = reserved_sectors.swap_bytes();
        self.bios_parameters.root_entries = root_entries.swap_bytes();
        self.bios_parameters.total_sectors_16 = total_sectors_16.swap_bytes();
        self.bios_parameters.media_type = media_type.swap_bytes();
        self.bios_parameters.sectors_per_fat_16 = sectors_per_fat_16.swap_bytes();
        self.bios_parameters.sectors_per_track = sectors_per_track.swap_bytes();
        self.bios_parameters.heads = heads.swap_bytes();
        self.bios_parameters.hidden_sectors = hidden_sectors.swap_bytes();
        self.bios_parameters.total_sectors_32 = total_sectors_32.swap_bytes();
        self.bios_parameters.sectors_per_fat_32 = sectors_per_fat_32.swap_bytes();
        self.bios_parameters.extended_flags = extended_flags.swap_bytes();
        self.bios_parameters.fs_version = fs_version.swap_bytes();
        self.bios_parameters.root_dir_first_cluster = root_dir_first_cluster.swap_bytes();
        self.bios_parameters.fs_info_sector = fs_info_sector.swap_bytes();
        self.bios_parameters.backup_boot_sector = backup_boot_sector.swap_bytes();
        self.bios_parameters.volume_id = volume_id.swap_bytes();
    }
}

//I am assuming the most common case here, many different standards exist,
//But this is the most common one still used today to my knowledge.
#[derive(Clone)]
#[repr(C, packed)]
pub struct BiosParameters {
    // DOS 2.0 BP
    bytes_per_sector: u16,
    sectors_per_cluster: u8,
    reserved_sectors: u16,
    fats: u8,
    root_entries: u16,
    total_sectors_16: u16,
    media_type: u8,
    sectors_per_fat_16: u16,

    // DOS 3.31 extensions
    sectors_per_track: u16,
    heads: u16,
    hidden_sectors: u32,
    total_sectors_32: u32,

    // FAT32 style Extended BIOS Parameter Block
    sectors_per_fat_32: u32,
    extended_flags: u16,
    fs_version: u16,
    root_dir_first_cluster: u32,
    fs_info_sector: u16,
    backup_boot_sector: u16,
    _0x29: [u8; 12],
    drive_num: u8,
    flags: StatusFlags,
    ext_sig: u8,
    volume_id: u32,
    volume_label: [u8; 11],
    fs_identity_label: [u8; 8],
}
#[derive(Debug, PartialEq)]
pub enum FSType {
    Fat12,
    Fat16,
    Fat32,
    ExFat,
}
impl FSType {
    const MIN_CLUSTERS_FAT16: u32 = 4085;
    const MIN_CLUSTERS_FAT32: u32 = 65525;
    const MAX_CLUSTERS_FAT32: u32 = 0x0FFF_FFF4;
    pub fn guess(sector_count: u32) -> Self {
        if sector_count < Self::MIN_CLUSTERS_FAT16 {
            FSType::Fat12
        } else if sector_count < Self::MIN_CLUSTERS_FAT32 {
            FSType::Fat16
        } else {
            FSType::Fat32
        }
    }
}

impl BiosParameters {
    pub const fn should_be_fat32(&self) -> bool {
        // Required by FAT32 (since it uses the 32-bit fields)
        // However don't know if used by other filesystems.
        self.sectors_per_fat_16 == 0
    }
    pub fn evaluate(&self) -> Result<FSType, BadFSError> {
        if self.fs_version == 0 {
            return Err(BadFSError::BadFSVersion);
        }
        //Verify bytes per sector
        let bps = self.bytes_per_sector;
        if !bps.is_power_of_two() {
            return Err(BadFSError::BadBPSPower);
        }
        if !(512..=4096).contains(&bps) {
            return Err(BadFSError::BadBPSCount);
        }
        //verify sectors per cluster
        if !self.sectors_per_cluster.is_power_of_two() {
            return Err(BadFSError::BadSPC);
        }
        //verify reserved sectors
        let reserved_sectors = self.reserved_sectors;
        if reserved_sectors < 1 {
            return Err(BadFSError::NoReservedSectors);
        }
        //verify fat tables
        if self.fats == 0 {
            return Err(BadFSError::NoFats);
        }
        if self.should_be_fat32() {
            if self.backup_boot_sector >= self.reserved_sectors {
                return Err(BadFSError::BadBackup);
            }
            if self.fs_info_sector >= self.reserved_sectors {
                return Err(BadFSError::BadFSInfo);
            }
            if self.root_entries != 0 {
                return Err(BadFSError::NoRootEntries);
            }
            if self.total_sectors_16 != 0 {
                return Err(BadFSError::BadSectorCountFat32);
            }
            if self.sectors_per_fat_32 == 0 {
                return Err(BadFSError::BadSPF);
            }
        } else {
            if self.root_entries == 0 {
                return Err(BadFSError::NoRootEntries);
            }
            if self.total_sectors_16 == 0 && self.total_sectors_32 == 0 {
                return Err(BadFSError::SectorCountZero);
            }
            if self.total_sectors_16 != 0 && self.total_sectors_32 != 0 {
                if self.total_sectors_16 as u32 != self.total_sectors_32 {
                    return Err(BadFSError::SectorCountMismatch);
                }
            }
        }
        let guess = FSType::guess(self.total_sectors());
        if self.should_be_fat32() && guess != FSType::Fat32 {
            return Err(BadFSError::BadFatSigns);
        }
        Ok(guess)
    }
    pub fn total_sectors(&self) -> u32 {
        if self.total_sectors_16 == 0 {
            self.total_sectors_32
        } else {
            self.total_sectors_16 as u32
        }
    }
}
pub enum BadFSError {
    //Filesystem version was not 0 (lmao)
    BadFSVersion,
    //Bytes per sector was not a power of 2
    BadBPSPower,
    //Bytes per sector was not in range [512..=4096]
    BadBPSCount,
    //Sectors per cluster was nnot a power of 2
    BadSPC,
    //Invalid ammount of reserved sectors (not in range of [1..])
    NoReservedSectors,
    //Invalid backup sector (not inside reserved sector range)
    BadBackup,
    //Invalid file system info sector (not inside reserved sector range)
    BadFSInfo,
    //ammount of fat tables reported as 0
    NoFats,
    //root entries reported as 0
    NoRootEntries,
    // invalid total sector count (nonzero 16-bit report on FAT32)
    BadSectorCountFat32,
    // invalid total sector count (reported as 0 everywhere)
    SectorCountZero,
    // invalid total sector count (conflicting reports)
    SectorCountMismatch,
    // Invalid sectors per fat (FAT32 only)
    BadSPF,
    // Filesystem is using non-standard identification method
    BadFatSigns,
    // Filesystem contains too many clusters
    TooManyClusters,
    // Boot sector signature is malformed
    InvalidBootSign,
}

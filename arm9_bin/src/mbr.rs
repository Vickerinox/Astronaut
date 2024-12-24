#[repr(u8)]
enum PartitionTypes {
    Empty = 0,
    Fat12 = 1,
    XENIXroot = 2,
    XENIXuser = 3,
    Fat16Small = 4,
    Extended = 5,
    Fat16Large = 6,
    ExFat = 7,
    LogicalSectoredFat = 8,
    Fat32CHS = 0x0B,
    Far32LBA = 0x0E,
}
#[derive(Debug)]
pub enum MBRError {
    BadBootstrap,
    BadPartitions,
    BadSignature,
}

#[repr(C)]
pub struct MBR {
    pub bootstrap: [u8; 446],
    pub partitions: [PartitionEntry; 4],
    pub boot_signature: [u8; 2],
}
#[repr(C, packed)]
pub struct PartitionEntry {
    status: u8,
    chs_start: CHS,
    partition_type: u8,
    chs_end: CHS,
    pub lba: u32,
    pub sector_count: u32,
}

pub struct CHS([u8; 3]);
impl core::fmt::Debug for CHS {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CHS")
            .field("raw", &self.0)
            .field("head", &(self.head()))
            .field("sector", &(self.sector()))
            .field("cylinder", &(self.cylinder()))
            .finish()
    }
}
impl CHS {
    pub fn head(&self) -> u8 {
        self.0[0]
    }
    pub fn sector(&self) -> u8 {
        self.0[1] & 0b111111
    }
    pub fn cylinder(&self) -> u16 {
        self.0[2] as u16 | ((self.0[1] as u16) & 0b11000000 << 2)
    }
}

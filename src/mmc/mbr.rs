use std::io::Error as IoError;
use std::io::Read;
pub trait ByteDecode: Sized {
    type Error;
    fn from_reads<R: Read>(reader: &mut R) -> Result<Self, Self::Error>;
}
impl ByteDecode for PartitionEntry {
    type Error = IoError;

    fn from_reads<R: Read>(reader: &mut R) -> Result<Self, Self::Error> {
        let status: [u8; 1] = read_direct(reader)?;
        let status = status[0];
        let chs_start = CHS(read_direct(reader)?);
        let partition_type: [u8; 1] = read_direct(reader)?;
        let partition_type = partition_type[0];
        let chs_end = CHS(read_direct(reader)?);
        let lba = u32::from_le_bytes(read_direct(reader)?);
        let sector_count = u32::from_le_bytes(read_direct(reader)?);
        Ok(PartitionEntry {
            status,
            chs_start,
            partition_type,
            chs_end,
            lba,
            sector_count,
        })
    }
}
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
    FailedBootstrapRead(IoError),
    FailedPartitionRead(IoError),
    BadSignature,
    FailedSignatureRead(IoError),
}
impl ByteDecode for MBR {
    type Error = MBRError;

    fn from_reads<R: Read>(reader: &mut R) -> Result<Self, Self::Error> {
        let bootstrap = read_direct(reader).map_err(|e| MBRError::FailedBootstrapRead(e))?;

        let partitions = core::array::try_from_fn(|_| PartitionEntry::from_reads(reader))
            .map_err(|e| MBRError::FailedPartitionRead(e))?;
        let boot_signature = read_direct(reader).map_err(|e| MBRError::FailedSignatureRead(e))?;
        if boot_signature != [0x55, 0xAA] {
            return Err(MBRError::BadSignature);
        }
        Ok(MBR {
            bootstrap,
            partitions,
            boot_signature,
        })
    }
}
fn read_direct<const N: usize, R: Read>(reader: &mut R) -> Result<[u8; N], IoError> {
    let mut buf = [0; N];
    match reader.read_exact(&mut buf) {
        Ok(()) => Ok(buf),
        Err(e) => Err(e),
    }
}
fn read_byte<R: Read>(reader: &mut R) -> Result<u8, IoError> {
    let buf: [u8; 1] = read_direct(reader)?;
    Ok(buf[0])
}
#[repr(C)]
#[derive(Debug)]
pub struct MBR {
    pub bootstrap: [u8; 446],
    pub partitions: [PartitionEntry; 4],
    pub boot_signature: [u8; 2],
}
#[repr(C)]
#[derive(Debug)]
pub struct PartitionEntry {
    status: u8,
    chs_start: CHS,
    partition_type: u8,
    chs_end: CHS,
    pub lba: u32,
    pub sector_count: u32,
}

pub struct CHS([u8; 3]);
impl std::fmt::Debug for CHS {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

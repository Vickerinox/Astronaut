use fatfs_embedded::fatfs::diskio::{self, *};

const STORAGE_SIZE: usize = 1024 * 1000 * 64; //Simulate a 64MB device
const SECTOR_SIZE: usize = 512;

pub struct RamBlockStorage {
    memory: Vec<u8>
}

impl RamBlockStorage {
    pub fn new() -> RamBlockStorage {
        Self {
            memory: Vec::new()
        }
    }
}

impl FatFsDriver for RamBlockStorage {
    fn disk_status(&self, _drive: u8) -> u8 {
        return 0
    }

    fn disk_initialize(&mut self, _drive: u8) -> u8 {
        self.memory.resize(STORAGE_SIZE, 0);
        return 0
    }

    fn disk_read(&mut self, _drive: u8, buffer: &mut [u8], sector: u32) -> diskio::DiskResult {
        let offset: usize = sector as usize * 512;
        buffer.copy_from_slice(self.memory[offset..offset+512].as_mut());
        DiskResult::Ok
    }

    fn disk_write(&mut self, _drive: u8, buffer: &[u8], sector: u32) -> diskio::DiskResult {
        let offset: usize = sector as usize * 512;
        self.memory[offset..offset+512].copy_from_slice(buffer);
        DiskResult::Ok
    }

    fn disk_ioctl(&self, data: &mut diskio::IoctlCommand) -> diskio::DiskResult {
        if let IoctlCommand::CtrlSync(_) = data {
            return DiskResult::Ok
        } else if let IoctlCommand::GetSectorCount(_) = data {
            let sector_count = self.memory.len() / SECTOR_SIZE;
            *data = IoctlCommand::GetSectorCount(sector_count as u32);
            return DiskResult::Ok
        } else if let IoctlCommand::GetSectorSize(_) = data {
            *data = IoctlCommand::GetSectorSize(SECTOR_SIZE as u16);
            return DiskResult::Ok
        } else if let IoctlCommand::GetBlockSize(_) = data {
            let erase_block_count = SECTOR_SIZE;
            *data = IoctlCommand::GetBlockSize(erase_block_count as u32);
            return DiskResult::Ok
        } else {
            return DiskResult::Error
        }
    }

    fn get_fattime(&self) -> chrono::prelude::NaiveDateTime {
        chrono::offset::Local::now().naive_local()
    }
}
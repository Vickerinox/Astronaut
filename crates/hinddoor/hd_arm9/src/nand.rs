use alloc::boxed::Box;
use reboot_lib::StorageSector;


pub struct BasicSDMMCCursor<'a> {
    buffer: &'a mut [StorageSector],
    offset: usize,
    buffer_sector: u32,
    lba: u32,
    nand: bool,
    dirty: bool,
}

pub struct AsyncReadHandle(Box<core::cell::UnsafeCell<AsyncSDMMCReadStatus>>);
pub enum AsyncSDMMCReadStatus {
    Pending,
    Error,
    FatalError,
    MediaMissing,
    Finished,
}
impl<'a> BasicSDMMCCursor<'a> {
    pub fn new(buffer: &'a mut [StorageSector], lba_sector: u32, is_nand: bool) -> Result<Self, u32> {
        let mut oneself = Self {
            buffer_sector: 0,
            buffer,
            offset: 0,
            lba: lba_sector,
            nand: is_nand,
            dirty: false,
        };
        oneself.read_sector(0)?;
        Ok(oneself)
    }
    pub fn read_sector(&mut self, sector: u32) -> Result<(), u32> {
        match self.nand {
            true => crate::read_encrypted_nand(self.buffer, self.lba + sector),
            false => crate::read_sd_card(self.buffer, self.lba + sector),
        }
    }
    pub fn write_sector(&mut self, sector: u32) -> Result<(), u32> {
        match self.nand {
            true => Err(123456789),
            false => crate::write_sd_card(self.buffer, self.lba + sector),
        }
    }
    fn advance_sector(&mut self) -> Result<(), BasicSDMMCError> {
        let advance = self.offset / 512;
        self.offset %= 512;
        self.flush()?;
        self.buffer_sector += advance as u32;
        match self.read_sector(self.buffer_sector) {
            Ok(_) => (),
            Err(_) => return Err(BasicSDMMCError::BadRead),
        }
        Ok(())
    }
    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, BasicSDMMCError> {
        let available_buffer = (self.buffer.len() * 512) - self.offset;
        let buffer_cutoff = available_buffer.min(buf.len());

        let byte_buffer = {
            let l = self.buffer.as_mut().len() << 9;
            let s = self.buffer.as_mut() as *mut [reboot_lib::StorageSector] as *mut u32 as *mut u8;
            unsafe { core::slice::from_raw_parts_mut(s, l) }
        };

        let Some(read) = buf.get_mut(..buffer_cutoff) else {
            return Err(BasicSDMMCError::BadCutoff);
        };
        let Some(our_buf) = byte_buffer.get(self.offset..self.offset + buffer_cutoff) else {
            return Err(BasicSDMMCError::BadLogic);
        };

        read.copy_from_slice(our_buf);
        self.offset += buffer_cutoff;
        if self.offset >= (self.buffer.len() * 512) {
            self.advance_sector()?;
        }

        Ok(buffer_cutoff)
    }
    pub fn write(&mut self, buf: &[u8]) -> Result<usize, BasicSDMMCError> {
        self.dirty = true;
        let available_buffer = (self.buffer.len() * 512) - self.offset;
        let buffer_cutoff = available_buffer.min(buf.len());

        let byte_buffer = {
            let l = self.buffer.as_mut().len() << 9;
            let s = self.buffer.as_mut() as *mut [reboot_lib::StorageSector] as *mut u32 as *mut u8;
            unsafe { core::slice::from_raw_parts_mut(s, l) }
        };

        let Some(read) = buf.get(..buffer_cutoff) else {
            return Err(BasicSDMMCError::BadCutoff);
        };
        let Some(our_buf) = byte_buffer.get_mut(self.offset..self.offset + buffer_cutoff) else {
            return Err(BasicSDMMCError::BadLogic);
        };

        our_buf.copy_from_slice(read);
        self.offset += buffer_cutoff;

        if self.offset >= (self.buffer.len() * 512) {
            self.advance_sector()?;
        }

        Ok(buffer_cutoff)
    }
    pub fn flush(&mut self) -> Result<(), BasicSDMMCError> {
        if self.dirty {
            let sect = self.buffer_sector;
            match self.write_sector(sect as u32) {
                Ok(_) => self.dirty = false,
                Err(_) => return Err(BasicSDMMCError::BadWrite),
            }
        }
        Ok(())
    }
    pub fn seek(&mut self, sector: u32) -> Result<u32, BasicSDMMCError> {
        if sector < self.buffer_sector || sector >= self.buffer_sector + self.buffer.len() as u32 {
            self.flush()?;
            self.offset = 0;
            self.buffer_sector = sector;
            match self.read_sector(self.buffer_sector) {
                Ok(_) => Ok(self.buffer_sector),
                Err(_) => Err(BasicSDMMCError::BadRead),
            }
        } else {
            let off = sector - self.buffer_sector;
            self.offset = off as usize * 512;
            Ok(self.buffer_sector + off)
        }
    }
}
#[derive(Debug, PartialEq)]
pub enum BasicSDMMCError {
    BadWrite,
    BadRead,
    BadLogic,
    BadCutoff,
}

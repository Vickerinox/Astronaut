use alloc::boxed::Box;
use reboot_lib::StorageSector;
pub struct BasicSDMMCCursor<'a> {
    buffer_virtual_position: u64,
    buffer: &'a mut [StorageSector],
    pos: u64,
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
    pub fn new(buffer: &'a mut [StorageSector], lba_sector: u32, is_nand: bool) -> Self {
        let mut oneself = Self {
            buffer_virtual_position: 0,
            buffer,
            pos: 0,
            lba: lba_sector,
            nand: is_nand,
            dirty: false,
        };
        oneself.read_sector(0);
        oneself
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
    fn switch_sector(&mut self) -> Result<(), u32> {
        let virtual_sector = self.pos / 512;
        self.flush().unwrap();
        match self.read_sector(virtual_sector as u32) {
            Ok(_) => self.buffer_virtual_position = virtual_sector * 512,
            Err(_) => return Err(123456789),
        }
        //assert!(self.pos >= self.buffer_virtual_position);
        //assert!(self.pos < self.buffer_virtual_position + (self.buffer.len() * 512) as u64);
        Ok(())
    }
    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, BasicSDMMCError> {
        let mut read_bytes = 0;
        //assert!(self.pos >= self.buffer_virtual_position);
        //assert!(self.pos < self.buffer_virtual_position + (self.buffer.len() * 512) as u64);
        let pos_in_buffer = (self.pos - self.buffer_virtual_position) as usize;
        let available_buffer = (self.buffer.len() * 512) - pos_in_buffer;
        let buffer_cutoff = available_buffer.min(buf.len());
        let (read, _remaining) = buf.split_at_mut(buffer_cutoff);

        let byte_buffer = {
            let l = self.buffer.as_mut().len() << 9;
            let s = self.buffer.as_mut() as *mut [reboot_lib::StorageSector] as *mut u32 as *mut u8;
            unsafe { core::slice::from_raw_parts_mut(s, l) }
        };

        read.copy_from_slice(&byte_buffer[pos_in_buffer..][..buffer_cutoff]);
        self.pos += buffer_cutoff as u64;
        read_bytes += buffer_cutoff;
        if self.pos >= self.buffer_virtual_position + (self.buffer.len() * 512) as u64 {
            self.switch_sector();
        }

        Ok(read_bytes)
    }
    pub fn write(&mut self, buf: &[u8]) -> Result<usize, BasicSDMMCError> {
        self.dirty = true;
        let mut read_bytes = 0;

        let pos_in_buffer = (self.pos - self.buffer_virtual_position) as usize;
        let available_buffer = (self.buffer.len() * 512) - pos_in_buffer;
        let buffer_cutoff = available_buffer.min(buf.len());
        let (read, _remaining) = buf.split_at(buffer_cutoff);

        let byte_buffer = {
            let l = self.buffer.as_mut().len() << 9;
            let s = self.buffer.as_mut() as *mut [reboot_lib::StorageSector] as *mut u32 as *mut u8;
            unsafe { core::slice::from_raw_parts_mut(s, l) }
        };

        byte_buffer[pos_in_buffer..][..buffer_cutoff].copy_from_slice(read);
        self.pos += buffer_cutoff as u64;
        read_bytes += buffer_cutoff;
        if self.pos >= self.buffer_virtual_position + (self.buffer.len() * 512) as u64 {
            self.switch_sector();
        }

        Ok(read_bytes)
    }
    pub fn flush(&mut self) -> Result<(), BasicSDMMCError> {
        if self.dirty {
            let sect = self.buffer_virtual_position / 512;
            self.write_sector(sect as u32).unwrap();
            self.dirty = false;
        }
        Ok(())
    }
    pub fn seek(&mut self, pos: u64) -> Result<u64, BasicSDMMCError> {
        self.pos = pos;
        if (self.pos >= self.buffer_virtual_position + (self.buffer.len() * 512) as u64)
            || (self.pos < self.buffer_virtual_position)
        {
            self.switch_sector();
        }
        Ok(self.pos)
    }
}
#[derive(Debug, PartialEq)]
pub enum BasicSDMMCError {
    UnexpectedEof,
    WriteZero,
    Unsupported,
}

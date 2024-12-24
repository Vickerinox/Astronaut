pub unsafe fn mount_twl_main(
    lba: u32,
    size: u32,
    buffer: &mut [reboot_lib::StorageSector],
) -> Result<
    fatfs::FileSystem<SDMMCCursor<&mut [reboot_lib::StorageSector], SDMMCAccessor, 9>>,
    fatfs::Error<SDMMCError>,
> {
    let cursor = SDMMCCursor::new(
        SDMMCAccessor {
            lba,
            size,
            nand_e: true,
        },
        buffer,
    );
    fatfs::FileSystem::new(cursor, fatfs::FsOptions::new())
}
pub unsafe fn mount_sd_card_partition(
    lba: u32,
    size: u32,
    buffer: &mut [reboot_lib::StorageSector],
) -> Result<
    fatfs::FileSystem<SDMMCCursor<&mut [reboot_lib::StorageSector], SDMMCAccessor, 9>>,
    fatfs::Error<SDMMCError>,
> {
    let cursor = SDMMCCursor::new(
        SDMMCAccessor {
            lba,
            size,
            nand_e: false,
        },
        buffer,
    );
    fatfs::FileSystem::new(cursor, fatfs::FsOptions::new())
}
pub struct SDMMCAccessor {
    lba: u32,
    size: u32,
    nand_e: bool,
}

impl SectorAccess<9> for SDMMCAccessor {
    fn read_sector(&mut self, sector: usize, buf: &mut [reboot_lib::StorageSector]) {
        if self.nand_e {
            crate::read_encrypted_nand(buf, sector as u32 + self.lba);
        } else {
            crate::read_sd_card(buf, sector as u32 + self.lba);
        }
    }

    fn write_sector(&mut self, sector: usize, buf: &[reboot_lib::StorageSector]) {}

    fn size(&mut self) -> usize {
        (self.size << 9) as usize
    }
}

pub struct SDMMCCursor<
    T: AsMut<[reboot_lib::StorageSector]>,
    I: SectorAccess<N>,
    const N: usize = 9,
> {
    buffer: T,
    buf_sector: usize,
    pos: usize,
    interface: I,
    flush_on_next: bool,
}
impl<T: AsMut<[reboot_lib::StorageSector]>, const N: usize, I: SectorAccess<N>>
    SDMMCCursor<T, I, N>
{
    /// Size of a block
    const BLOCK_SIZE: usize = (1 << N);
    /// Create a new instance of this struct
    ///
    /// Note: Preloads block 0 for optimization
    ///
    /// Panics if `block buffer`'s length isn't equal to BLOCK_SIZE (i.e 1<<N)
    pub fn new(mut interface: I, mut block_buffer: T) -> Self {
        let block_buffer_mut = block_buffer.as_mut();
        assert_eq!(block_buffer_mut.len() << 9, Self::BLOCK_SIZE);
        interface.read_sector(0, block_buffer_mut);
        Self {
            buffer: block_buffer,
            interface,
            buf_sector: 0,
            pos: 0,
            flush_on_next: false,
        }
    }
    /// switches out self.buffer with the contents in block `sector`.
    ///
    /// Writes the current buffer back to nand if it has been modified.
    ///
    /// Does nothing if self.buf_sector is equal to `sector`
    fn switch_sector(&mut self, sector: usize) {
        //triggers when the old current sector has been modified
        if self.buf_sector != sector {
            self.write_loaded_sector();
            self.load_sector(sector);
        }
    }
    /// Write the currently loaded block back onto the NAND
    ///
    /// Does nothing if `self.flush_on_next` is false
    /// as no data should have changed.
    fn write_loaded_sector(&mut self) {
        if self.flush_on_next {
            self.interface
                .write_sector(self.buf_sector, &self.buffer.as_mut());
            self.flush_on_next = false;
        }
    }

    fn load_sector(&mut self, sector: usize) {
        self.buf_sector = sector;
        self.interface.read_sector(sector, self.buffer.as_mut());
    }

    /// loads the sector which self.pos is currently on
    ///
    /// Returns the offset into the `self.buffer` which `self.pos` lands on.
    /// as well as the maximum length possible to read and write into
    /// `self.buffer` until either `self.buffer` runs out or max_len is reached.
    fn load_sector_and_offsets(&mut self, max_len: usize) -> (usize, usize) {
        let pos_sector = self.pos >> N;
        //switch to new sector
        self.switch_sector(pos_sector);
        //find where self.pos indexes into the currently loaded buffer
        let offset = self.pos & (Self::BLOCK_SIZE - 1);
        //find where buf_len or self.buffer runs out.
        let max_len = (Self::BLOCK_SIZE - offset).min(max_len);
        (offset as usize, max_len)
    }
}
impl<T: AsMut<[reboot_lib::StorageSector]>, const N: usize, I: SectorAccess<N>> fatfs::Read
    for SDMMCCursor<T, I, N>
{
    fn read(&mut self, mut buf: &mut [u8]) -> Result<usize, SDMMCError> {
        let mut reads = 0;
        while buf.len() > 0 {
            let (offset, len) = self.load_sector_and_offsets(buf.len());
            let (data_buf, rest_buf) = buf.split_at_mut(len);
            unsafe {
                let l = self.buffer.as_mut().len() << 9;
                let s =
                    self.buffer.as_mut() as *mut [reboot_lib::StorageSector] as *mut u32 as *mut u8;
                let bytes = core::slice::from_raw_parts_mut(s, l);
                data_buf.copy_from_slice(&bytes[offset..][..len]);
            }
            buf = rest_buf;
            reads += len;
            self.pos += len;
        }
        Ok(reads)
    }
}
impl<T: AsMut<[reboot_lib::StorageSector]>, const N: usize, I: SectorAccess<N>> fatfs::Write
    for SDMMCCursor<T, I, N>
{
    fn write(&mut self, mut buf: &[u8]) -> Result<usize, SDMMCError> {
        let mut writes = 0;
        while buf.len() > 0 {
            let (offset, len) = self.load_sector_and_offsets(buf.len());
            let (data, rest_buf) = buf.split_at(len);
            unsafe {
                let l = self.buffer.as_mut().len() << 9;
                let s =
                    self.buffer.as_mut() as *mut [reboot_lib::StorageSector] as *mut u32 as *mut u8;
                let bytes = core::slice::from_raw_parts_mut(s, l);
                bytes[offset..][..len].copy_from_slice(data);
            }
            self.flush_on_next = true;
            buf = rest_buf;
            writes += len;
            self.pos += len;
        }
        Ok(writes)
    }
    fn flush(&mut self) -> Result<(), SDMMCError> {
        return Err(SDMMCError);
        self.write_loaded_sector();
        Ok(())
    }
}
#[derive(Debug)]
pub struct SDMMCError;
impl<T: AsMut<[reboot_lib::StorageSector]>, const N: usize, I: SectorAccess<N>> fatfs::IoBase
    for SDMMCCursor<T, I, N>
{
    type Error = SDMMCError;
}
impl fatfs::IoError for SDMMCError {
    fn is_interrupted(&self) -> bool {
        false
    }

    fn new_unexpected_eof_error() -> Self {
        SDMMCError
    }

    fn new_write_zero_error() -> Self {
        SDMMCError
    }
}
impl<T: AsMut<[reboot_lib::StorageSector]>, const N: usize, I: SectorAccess<N>> fatfs::Seek
    for SDMMCCursor<T, I, N>
{
    fn seek(&mut self, pos: fatfs::SeekFrom) -> Result<u64, SDMMCError> {
        use fatfs::SeekFrom;
        let (base_pos, offset) = match pos {
            SeekFrom::Start(n) => {
                self.pos = n as usize;
                return Ok(n);
            }
            SeekFrom::End(n) => (self.interface.size(), n),
            SeekFrom::Current(n) => (self.pos, n),
        };
        match base_pos.checked_add_signed(offset as isize) {
            Some(n) => {
                self.pos = n;
                Ok(self.pos as u64)
            }
            None => Err(SDMMCError),
        }
    }
}
impl<T: AsMut<[reboot_lib::StorageSector]>, const N: usize, I: SectorAccess<N>> Drop
    for SDMMCCursor<T, I, N>
{
    fn drop(&mut self) {
        self.write_loaded_sector();
    }
}
pub struct NandWrapper<T: AsMut<[u8]>, const N: usize = 9>(T);
impl<T: AsMut<[u8]>> NandWrapper<T, 9> {
    pub fn new(buffer: T) -> Self {
        Self(buffer)
    }
}
/// Accessing NAND data with a constant block size, where blocksize is 1<<N.
///
/// For standard block size (512 bytes) use N = 9
///
/// The functions should not encrypt or decrypt the data.
/// This should instead be handled by further interfaces.
pub trait SectorAccess<const N: usize> {
    fn read_sector(&mut self, sector: usize, buf: &mut [reboot_lib::StorageSector]);
    fn write_sector(&mut self, sector: usize, buf: &[reboot_lib::StorageSector]);
    fn size(&mut self) -> usize;
}

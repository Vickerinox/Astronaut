use std::io::{Read, Seek, Write};

use crypto::{
    aes,
    aes::KeySize,
    blockmodes::NoPadding,
    buffer::{RefReadBuffer, RefWriteBuffer},
};
pub struct NandSectorCursor<T: AsMut<[u8]>, I: NandSectorAccess<N>, const N: usize = 9> {
    ctr_base: u128,
    aes_key: u128,

    buffer: T,
    buf_sector: usize,
    pos: usize,

    interface: I,

    flush_on_next: bool,
    encrypted: bool,
}
impl<T: AsMut<[u8]>, const N: usize, I: NandSectorAccess<N>> NandSectorCursor<T, I, N> {
    /// Size of a block
    const BLOCK_SIZE: usize = (1 << N);
    /// The amount of bitshifts needed to convert self.buf_sector to a CTR offset.
    const CTR_SHIFT: usize = (N - 4);
    /// Create a new instance of this struct
    ///
    /// Note: Preloads block 0 for optimization
    ///
    /// Panics if `block buffer`'s length isn't equal to BLOCK_SIZE (i.e 1<<N)
    pub fn new(mut interface: I, mut block_buffer: T, ctr_base: u128, aes_key: u128) -> Self {
        let block_buffer_mut = block_buffer.as_mut();
        assert_eq!(block_buffer_mut.len(), Self::BLOCK_SIZE);
        interface.read_sector(0, block_buffer_mut);
        Self {
            buffer: block_buffer,
            interface,
            buf_sector: 0,
            ctr_base,
            aes_key,
            pos: 0,
            flush_on_next: false,
            encrypted: true,
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
            self.encrypt_buffer();
            self.interface
                .write_sector(self.buf_sector, &self.buffer.as_mut());
            self.flush_on_next = false;
        }
    }

    fn load_sector(&mut self, sector: usize) {
        self.encrypted = true;
        self.buf_sector = sector;
        self.interface.read_sector(sector, self.buffer.as_mut());
    }

    /// Encrypt the contents in self.buffer
    ///
    /// If contents are already encrypted this does nothing.
    fn encrypt_buffer(&mut self) {
        if !self.encrypted {
            self.crypt_buffer();
            self.encrypted = true;
        }
    }
    /// Decrypt the contents in self.buffer
    ///
    /// If contents are already decrypted this does nothing.
    fn decrypt_buffer(&mut self) {
        if self.encrypted {
            self.crypt_buffer();
            self.encrypted = false;
        }
    }
    /// USE DECRYPT OR ENCRYPT, NOT THIS.
    ///
    /// encrypts or decrypts `self.buffer`
    fn crypt_buffer(&mut self) {
        let ctr = self.ctr_base + (self.buf_sector << Self::CTR_SHIFT) as u128;
        for (i, slice) in self.buffer.as_mut().chunks_exact_mut(16).enumerate() {
            crypt_block(slice, &self.aes_key, ctr + i as u128);
        }
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
        //the current buffer may be encrypted by a flush or loading a new sector
        self.decrypt_buffer();
        //find where self.pos indexes into the currently loaded buffer
        let offset = self.pos & (Self::BLOCK_SIZE - 1);
        //find where buf_len or self.buffer runs out.
        let max_len = (Self::BLOCK_SIZE - offset).min(max_len);
        (offset as usize, max_len)
    }
}
impl<T: AsMut<[u8]>, const N: usize, I: NandSectorAccess<N>> Read for NandSectorCursor<T, I, N> {
    fn read(&mut self, mut buf: &mut [u8]) -> std::io::Result<usize> {
        let mut reads = 0;
        while buf.len() > 0 {
            let (offset, len) = self.load_sector_and_offsets(buf.len());
            let (data_buf, rest_buf) = buf.split_at_mut(len);
            data_buf.copy_from_slice(&self.buffer.as_mut()[offset..][..len]);
            buf = rest_buf;
            reads += len;
            self.pos += len;
        }
        Ok(reads)
    }
}
impl<T: AsMut<[u8]>, const N: usize, I: NandSectorAccess<N>> Write for NandSectorCursor<T, I, N> {
    fn write(&mut self, mut buf: &[u8]) -> std::io::Result<usize> {
        let mut writes = 0;
        while buf.len() > 0 {
            let (offset, len) = self.load_sector_and_offsets(buf.len());
            let (data, rest_buf) = buf.split_at(len);
            self.buffer.as_mut()[offset..][..len].copy_from_slice(data);
            self.flush_on_next = true;
            buf = rest_buf;
            writes += len;
            self.pos += len;
        }
        Ok(writes)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.write_loaded_sector();
        Ok(())
    }
}
impl<T: AsMut<[u8]>, const N: usize, I: NandSectorAccess<N>> Seek for NandSectorCursor<T, I, N> {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        use std::io::SeekFrom;
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
            None => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "tried seeking to before 0",
            )),
        }
    }
}
impl<T: AsMut<[u8]>, const N: usize, I: NandSectorAccess<N>> Drop for NandSectorCursor<T, I, N> {
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
pub trait NandSectorAccess<const N: usize> {
    fn read_sector(&mut self, sector: usize, buf: &mut [u8]);
    fn write_sector(&mut self, sector: usize, buf: &[u8]);
    fn size(&mut self) -> usize;
}
impl<T: AsMut<[u8]>, const N: usize> NandSectorAccess<N> for NandWrapper<T, N> {
    fn read_sector(&mut self, sector: usize, buf: &mut [u8]) {
        assert_eq!(buf.len(), 1 << N);
        let address = sector << N;
        buf.copy_from_slice(&self.0.as_mut()[address..][..buf.len()]);
    }

    fn write_sector(&mut self, sector: usize, buf: &[u8]) {
        assert_eq!(buf.len(), 1 << N);
        let address = sector << N;
        self.0.as_mut()[address..][..buf.len()].copy_from_slice(&buf);
    }

    fn size(&mut self) -> usize {
        self.0.as_mut().len()
    }
}

// encrypts or decrypts a single block of NAND data in place.
fn crypt_block(data: &mut [u8], key: &u128, ctr: u128) {
    assert_eq!(data.len(), 16);
    let keystream = {
        let mut scratch = [0u8; 16];
        aes::ecb_encryptor(KeySize::KeySize128, &key.to_be_bytes(), NoPadding)
            .encrypt(
                &mut RefReadBuffer::new(&ctr.to_be_bytes()),
                &mut RefWriteBuffer::new(&mut scratch),
                true,
            )
            .expect("keys and data are already always 16 bytes.");
        scratch.reverse();
        scratch
    };

    for (data, crypt) in data.iter_mut().zip(keystream) {
        *data ^= crypt
    }
}

#[test]
fn test_dsi() {
    let mut data = 0x00000000_00000000_00000000_00000000u128.to_le_bytes();
    crypt_block(
        &mut data,
        &0xb6da239c6b70c527dfefaba404120fe0,
        0xef23fa604ed21410c19229e9_95af8837 + 1,
    );
    println!("{data:02x?} {}", true as u32);
}

use alloc::vec::Vec;

use crate::boot::read_all;


#[repr(C)]
#[derive(Debug, Clone)]
pub struct BMPHeader {
    pub identity: u16,
    pub size: u32,
    pub reserved: u32,
    pub start_offset: u32,
}
#[repr(C)]
#[derive(Debug, Clone)]
pub struct DIBHeader40Bytes {
    pub size: u32,
    pub width: i32,
    pub height: i32,
    pub planes: u16,
    pub bits_per_pixel: u16,
    pub compression: u32,
    pub bitmap_size: u32,
    pub resolution_x: u32,
    pub resolution_y: u32,
    pub color_count: u32,
    pub important_colors: u32,
}

#[derive(Debug, Clone)]
pub struct DecodedBMP {
    pub header: BMPHeader,
    pub dib: DIBHeader40Bytes,
    pub colors: Vec<u8>,
    pub bitmap: Vec<u8>,
}
impl DecodedBMP {
    pub fn bitmap(&self) -> &[u8] {
        &self.bitmap
    }
    pub fn palette_table(&self) -> &[u8] {
        &self.colors
    }
    pub fn width(&self) -> usize {
        self.dib.width as usize
    }
    pub fn height(&self) -> usize {
        self.dib.height as usize
    }
}
impl BMPHeader {
    pub fn from_reader(fil: &mut fatfs_embedded::fatfs::File) -> Option<Self> {
        let mut buffer = [0u8; 14];
        if read_all(&mut buffer, fil).is_err() {
            return None;
        }
        let mut reader = buffer.iter().copied();
        let identity = read_u16(&mut reader);
        let size = read_u32(&mut reader);
        let reserved = read_u32(&mut reader);
        let start_offset = read_u32(&mut reader);
        Some(BMPHeader {
            identity,
            size,
            reserved,
            start_offset,
        })
    }
}
fn read_u32(mut iter: impl Iterator<Item = u8>) -> u32 {
    u32::from_le_bytes(core::array::from_fn(|_| iter.next().unwrap_or_default()))
}
fn read_u16(mut iter: impl Iterator<Item = u8>) -> u16 {
    u16::from_le_bytes(core::array::from_fn(|_| iter.next().unwrap_or_default()))
}
impl DIBHeader40Bytes {
    pub fn from_reader(fil: &mut fatfs_embedded::fatfs::File) -> Option<Self> {
        let mut buffer = [0u8; 40];
        if read_all(&mut buffer, fil).is_err() {
            return None;
        }
        let mut reader = buffer.iter().copied();
        let size = read_u32(&mut reader);
        let width = read_u32(&mut reader) as i32;
        let height = read_u32(&mut reader) as i32;
        let planes = read_u16(&mut reader);
        let bits_per_pixel = read_u16(&mut reader);
        let compression = read_u32(&mut reader);
        let bitmap_size = read_u32(&mut reader);
        let resolution_x = read_u32(&mut reader);
        let resolution_y = read_u32(&mut reader);
        let color_count = read_u32(&mut reader);
        let important_colors = read_u32(&mut reader);
        Some(DIBHeader40Bytes {
            size,
            width,
            height,
            planes,
            bits_per_pixel,
            compression,
            bitmap_size,
            resolution_x,
            resolution_y,
            color_count,
            important_colors,
        })
    }
}

impl DecodedBMP {
    pub fn from_reader(mut reader: fatfs_embedded::fatfs::File) -> Option<DecodedBMP> {
        let header = BMPHeader::from_reader(&mut reader)?;
        let dib = DIBHeader40Bytes::from_reader(&mut reader)?;
        let palette_len = if dib.compression != 3 {
            dib.color_count.min(1 << dib.bits_per_pixel as u32) as usize
        } else {
            3
        };
        let mut colors = alloc::vec![0u8; palette_len*4];
        if read_all(&mut colors[..], &mut reader).is_err() {
            return None;
        };
        if fatfs_embedded::seek(&mut reader, header.start_offset).is_err() {
            return None;
        }
        let bitmap_size = dib.bitmap_size as usize;
        let mut bitmap = alloc::vec![0u8; bitmap_size];
        if read_all(&mut bitmap[..], &mut reader).is_err() {
            return None;
        };
        Some(DecodedBMP {
            header,
            dib,
            colors,
            bitmap,
        })
    }
}

/*
/// A 16-bit rgb triplet with 1-bit alpha
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct RGBA5(u16);

impl RGBA5 {
    const TRANSPARENT: Self = Self(0x80);

    pub const fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self((r as u16 & 0b11111) | ((g as u16 & 0b11111) << 5) | ((b as u16 & 0b11111) << 10))
    }
    pub const fn from_rgb_normalized(r: u8, g: u8, b: u8) -> Self {
        Self(
            ((r as u16 & 0b11111000) >> 3)
                | ((g as u16 & 0b11111000) << 2)
                | ((b as u16 & 0b11111000) << 7),
        )
    }
    pub const fn from_u16(color: u16) -> Self {
        Self(color)
    }
    pub const fn to_u16(&self) -> u16 {
        self.0
    }
}
*/

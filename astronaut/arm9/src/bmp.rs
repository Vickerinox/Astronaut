// SPDX-FileCopyrightText: 2026 Viktor Karlsson <viktor@koda.re>
// SPDX-License-Identifier: GPL-3.0-or-later

use alloc::vec::Vec;

use crate::boot::read_all;
use reboot_lib::fatfs_embedded;

unsafe impl reboot_lib::bytemuck::NoUninit for BMPHeader {}
unsafe impl reboot_lib::bytemuck::NoUninit for DIBHeader40Bytes {}

unsafe impl reboot_lib::bytemuck::Zeroable for BMPHeader {}
unsafe impl reboot_lib::bytemuck::Zeroable for DIBHeader40Bytes {}

unsafe impl reboot_lib::bytemuck::AnyBitPattern for BMPHeader {}
unsafe impl reboot_lib::bytemuck::AnyBitPattern for DIBHeader40Bytes {}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Default)]
pub struct BMPHeader {
    /// The value "BM"
    pub identity: u16,
    /// Size of the whole BMP file in Bytes
    pub size: u32,
    /// Reserved field 0
    pub reserved0: u16,
    /// Reserved field 1
    pub reserved1: u16,
    /// Offset to the start of the bitmap data
    pub start_offset: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct DIBHeader40Bytes {
    /// Size of the header (should be 40)
    pub size: u32,
    /// Width of the bitmap
    pub width: i32,
    /// Height of the bitmap
    pub height: i32,
    /// Color plane count (should be 1)
    pub planes: u16,
    /// Bits per pixel
    pub bits_per_pixel: u16,
    /// Compression method
    pub compression: u32,
    /// Size of the bitmap data in bytes (can be 0 for uncompressed bitmaps?)
    pub bitmap_size: u32,
    /// Horizontal pixels per meter
    pub resolution_x: u32,
    /// Vertical pixels per meter
    pub resolution_y: u32,
    /// Length of color table list
    pub color_count: u32,
    /// Number of important colors (in modern times, usually 0)
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
        let mut header = BMPHeader::default();
        read_all(reboot_lib::bytemuck::bytes_of_mut(&mut header), fil)
            .is_ok()
            .then_some(header)
    }
}
impl DIBHeader40Bytes {
    pub fn from_reader(fil: &mut fatfs_embedded::fatfs::File) -> Option<Self> {
        let mut buffer = DIBHeader40Bytes::default();
        read_all(reboot_lib::bytemuck::bytes_of_mut(&mut buffer), fil).ok()?;

        // Toss out BMPs that are too large to be reasonable.
        // 1024x1024 is the maximum allowed to be used in a single texture on the DS, but this will do for now.
        if buffer.width > 256 || buffer.height > 192 {
            return None;
        }
        // And if all else fails, no single image should be larger than 1MB anyway. (that doesn't even fit in VRAM!)
        if buffer.bitmap_size > (1 << 20) {
            return None;
        }
        Some(buffer)
    }
    pub fn palette_len(&self) -> usize {
        if self.compression != 3 {
            self.color_count.min(1 << self.bits_per_pixel as u32) as usize
        } else {
            3
        }
    }
}
impl DecodedBMP {
    pub fn from_reader(mut reader: fatfs_embedded::fatfs::File) -> Option<DecodedBMP> {
        let header = BMPHeader::from_reader(&mut reader)?;
        let dib = DIBHeader40Bytes::from_reader(&mut reader)?;
        let palette_len = dib.palette_len();
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

pub enum AsyncBMPLoaderError {
    FailedToReadHeader,
    FailedToReadDIB,
    FailedToReadPalette,
    FailedToReadBitmap,
    FailedToSeekFile,
    NoLoadInProgress,
}
pub struct AsyncBMPLoader {
    current_load: Option<(fatfs_embedded::fatfs::File, DecodedBMP)>,
}

impl AsyncBMPLoader {
    pub fn new() -> Self {
        Self { current_load: None }
    }
    #[link_section = ".text_aux"]
    pub fn start_load(
        &mut self,
        mut reader: fatfs_embedded::fatfs::File,
    ) -> Result<DIBHeader40Bytes, AsyncBMPLoaderError> {
        let header =
            BMPHeader::from_reader(&mut reader).ok_or(AsyncBMPLoaderError::FailedToReadHeader)?;
        let dib = DIBHeader40Bytes::from_reader(&mut reader)
            .ok_or(AsyncBMPLoaderError::FailedToReadDIB)?;

        let palette_len = dib.palette_len();
        let bitmap_size = dib.bitmap_size as usize;

        self.current_load = Some((
            reader,
            DecodedBMP {
                header,
                dib,
                colors: Vec::with_capacity(palette_len * 4),
                bitmap: Vec::with_capacity(bitmap_size),
            },
        ));
        Ok(dib)
    }
    #[link_section = ".text_aux"]
    pub fn continue_load(
        &mut self,
        mut max_bytes: usize,
    ) -> Result<Option<DecodedBMP>, AsyncBMPLoaderError> {
        if let Some((reader, bmp)) = &mut self.current_load {
            let palette_len = bmp.dib.palette_len() * 4;
            let bitmap_size = bmp.dib.bitmap_size as usize;
            if bmp.colors.len() < palette_len {
                let remaining = palette_len - bmp.colors.len();
                let to_read = remaining.min(max_bytes);
                let start = bmp.colors.len();
                bmp.colors.resize(start + to_read, 0);
                read_all(&mut bmp.colors[start..], reader)
                    .map_err(|_| AsyncBMPLoaderError::FailedToReadPalette)?;
                max_bytes -= to_read;
            }
            if bmp.colors.len() == palette_len && max_bytes > 0 {
                if bmp.bitmap.len() < bitmap_size {
                    if fatfs_embedded::seek(
                        reader,
                        bmp.header.start_offset + bmp.bitmap.len() as u32,
                    )
                    .is_err()
                    {
                        return Err(AsyncBMPLoaderError::FailedToSeekFile);
                    }
                    let remaining = bitmap_size - bmp.bitmap.len();
                    let to_read = remaining.min(max_bytes);
                    let start = bmp.bitmap.len();
                    bmp.bitmap.resize(start + to_read, 0);
                    read_all(&mut bmp.bitmap[start..], reader)
                        .map_err(|_| AsyncBMPLoaderError::FailedToReadBitmap)?;
                }
            }
            if bmp.bitmap.len() == bitmap_size {
                let bmp = self.current_load.take().unwrap().1;
                Ok(Some(bmp))
            } else {
                Ok(None)
            }
        } else {
            Err(AsyncBMPLoaderError::NoLoadInProgress)
        }
    }
    #[link_section = ".text_aux"]
    pub fn abort_load(&mut self) {
        self.current_load = None;
    }
}

use std::{
    error::Error,
    io::{Read, Seek},
};

fn read_bytevec(mut reader: impl Read, len: usize) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut buffer = vec![0u8; len];
    reader.read_exact(&mut buffer)?;
    Ok(buffer)
}
fn read_bytebuffer<const N: usize>(mut reader: impl Read) -> Result<[u8; N], Box<dyn Error>> {
    let mut buffer = [0u8; N];
    reader.read_exact(&mut buffer)?;
    Ok(buffer)
}
fn _read_u8<R: Read>(reader: R) -> Result<u8, Box<dyn Error>> {
    read_bytebuffer::<1>(reader).map(|v| v[0])
}
fn read_u16<R: Read>(reader: R) -> Result<u16, Box<dyn Error>> {
    read_bytebuffer::<2>(reader).map(|v| u16::from_le_bytes(v))
}
fn read_u32<R: Read>(reader: R) -> Result<u32, Box<dyn Error>> {
    read_bytebuffer::<4>(reader).map(|v| u32::from_le_bytes(v))
}

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
    pub colors: Vec<[u8; 4]>,
    pub bitmap: Vec<u8>,
}
impl DecodedBMP {
    pub fn bitmap(&self) -> &[u8] {
        &self.bitmap
    }
    pub fn palette_table(&self) -> &[[u8; 4]] {
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
    pub fn from_reader<R: Read>(mut reader: R) -> Result<Self, Box<dyn Error>> {
        let identity = read_u16(&mut reader)?;
        let size = read_u32(&mut reader)?;
        let reserved = read_u32(&mut reader)?;
        let start_offset = read_u32(&mut reader)?;
        Ok(BMPHeader {
            identity,
            size,
            reserved,
            start_offset,
        })
    }
}
impl DIBHeader40Bytes {
    pub fn from_reader<R: Read>(mut reader: R) -> Result<Self, Box<dyn Error>> {
        let size = read_u32(&mut reader)?;
        let width = read_u32(&mut reader)? as i32;
        let height = read_u32(&mut reader)? as i32;
        let planes = read_u16(&mut reader)?;
        let bits_per_pixel = read_u16(&mut reader)?;
        let compression = read_u32(&mut reader)?;
        let bitmap_size = read_u32(&mut reader)?;
        let resolution_x = read_u32(&mut reader)?;
        let resolution_y = read_u32(&mut reader)?;
        let color_count = read_u32(&mut reader)?;
        let important_colors = read_u32(&mut reader)?;
        Ok(DIBHeader40Bytes {
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
    pub fn from_reader<R: Read + Seek>(mut reader: R) -> Result<DecodedBMP, Box<dyn Error>> {
        let header = BMPHeader::from_reader(&mut reader)?;
        let dib = DIBHeader40Bytes::from_reader(&mut reader)?;
        let palette_len = dib.color_count.min(1 << dib.bits_per_pixel as u32) as usize;
        let mut colors = Vec::with_capacity(palette_len);

        reader.seek(std::io::SeekFrom::Start(14 + dib.size as u64))?;
        for _ in 0..palette_len {
            colors.push(read_bytebuffer(&mut reader)?);
        }
        reader.seek(std::io::SeekFrom::Start(header.start_offset as u64))?;
        let bitmap_size = dib.bitmap_size as usize;
        let bitmap = read_bytevec(&mut reader, bitmap_size)?;
        Ok(DecodedBMP {
            header,
            dib,
            colors,
            bitmap,
        })
    }
}

#[derive(Debug)]
pub enum Block {
    Uncompressed(u8),
    Compressed { disp: u16, len: u8 },
}
fn decomp(data: &[u8]) -> Vec<Block> {
    let mut output = Vec::new();
    let mut pos = 0;
    while pos < data.len() {
        let (disp, len) = find_longest_match(data, pos);
        if len < 3 {
            output.push(Block::Uncompressed(data[pos]));
            pos += 1;
        } else {
            output.push(Block::Compressed {
                disp: disp as u16,
                len: len as u8,
            });
            pos += len;
        }
    }
    output
}
pub fn compress(data: &[u8]) -> Vec<u8> {
    let output: Vec<u8> = decomp(data)
        .chunks(8)
        .map(|i| {
            let mut output = vec![0u8];

            for block in i {
                output[0] <<= 1;
                match block {
                    Block::Uncompressed(byte) => {
                        output.push(*byte);
                    }
                    Block::Compressed { disp, len } => {
                        output[0] |= 1;
                        let len = len - 3;
                        let [hi, lo] = disp.to_be_bytes();
                        let hi = (hi & 0xF) | (len << 4);
                        output.push(hi);
                        output.push(lo);
                    }
                }
            }
            output[0] <<= 8 - i.len();
            output
        })
        .flatten()
        .collect();
    let mut final_output = (((data.len() as u32) << 8) | 0x10).to_le_bytes().to_vec();
    final_output.extend_from_slice(&output);
    while final_output.len() & 3 != 0 {
        final_output.push(0);
    }
    final_output
}

const MAX_DIST: usize = 0x1000;
const MAX_LEN: usize = 3 + 0xF;
// pos is current position in input value
fn find_longest_match(data: &[u8], pos: usize) -> (usize, usize) {
    let mut best_offset = 0;
    let mut best_len = 0;
    let start = if pos < MAX_DIST { 0 } else { pos - MAX_DIST };

    for offset in start..pos {
        let len = matching_len(data, offset, pos);
        if len >= best_len {
            best_offset = pos - (offset as usize) - 1;
            best_len = len;
        }
    }
    return (best_offset, best_len);
}

fn matching_len(data: &[u8], mut offset: usize, mut pos: usize) -> usize {
    let mut len = 0;
    while pos < data.len() && data[offset] == data[pos] && len < MAX_LEN {
        offset += 1;
        pos += 1;
        len += 1;
    }
    return len;
}

fn _decomp_parse(data: &[u8]) -> Vec<Block> {
    let mut output = Vec::new();
    let mut iter = data.into_iter();

    while let Some(mut flags) = iter.next().copied() {
        for _ in 0..8 {
            flags = flags.rotate_left(1);
            if flags & 1 == 0 {
                let Some(next_byte) = iter.next().copied() else {
                    break;
                };
                output.push(Block::Uncompressed(next_byte));
            } else {
                let Some(hi_byte) = iter.next().copied() else {
                    break;
                };
                let Some(lo_byte) = iter.next().copied() else {
                    break;
                };

                let len = 3 + (hi_byte >> 4);
                let disp = 1 + u16::from_be_bytes([hi_byte & 0xf, lo_byte]) as usize;

                output.push(Block::Compressed {
                    disp: disp as u16,
                    len: len as u8,
                });
            }
        }
    }
    output
}
fn _decompress(data: &[u8]) -> Vec<u8> {
    let signature = data[0];
    let size = u32::from_le_bytes([data[1], data[2], data[3], 0]) as usize;
    assert!(signature == 16);
    let decomp = _decomp_parse(&data[4..]);

    let mut output = Vec::new();
    for block in decomp {
        match block {
            Block::Uncompressed(byte) => output.push(byte),
            Block::Compressed { disp, len } => {
                for _ in 0..len {
                    output.push(output[output.len() - disp as usize]);
                }
            }
        }
    }
    assert_eq!(output.len(), size);
    output
}
pub fn generate_font(bmp: &[u8]) -> Option<Vec<u8>> {
    let font = DecodedBMP::from_reader(std::io::Cursor::new(bmp)).expect("INVALID FONT BMP!!!");
    assert!(font.colors.len() <= 8);
    assert!(font.colors.len() >= 2);
    assert!(font.width() == 1024);
    assert!(font.height() == 8);
    assert!(font.dib.compression == 0);
    //assert!(font.dib.size == 40, "{}", &font.dib.size);

    let bitmap: Vec<u8> = match font.dib.bits_per_pixel {
        4 => font
            .bitmap()
            .chunks_exact(2)
            .map(|i| {
                let Some([e, f]) = i.first_chunk().cloned() else {
                    unreachable!()
                };
                let a = ((e & 0x03) >> 0) << 2;
                let b = ((e & 0x30) >> 4) << 0;
                let c = ((f & 0x03) >> 0) << 6;
                let d = ((f & 0x30) >> 4) << 4;
                a | b | c | d
            })
            .collect(),
        count => {
            panic!("unsupported bits per pixel count for font bmp, {count}, (use 4-bit bmp's)")
        }
    };
    let mut bitmap: Vec<u8> = bitmap.chunks_exact(256).rev().flatten().cloned().collect();
    let colors = font
        .colors
        .iter()
        .map(|i| {
            let [b, g, r, _] = i.clone();
            let r = ((r >> 3) as u16) << 0;
            let g = ((g >> 3) as u16) << 5;
            let b = ((b >> 3) as u16) << 10;
            (r | g | b).to_le_bytes()
            //0xffffu16.to_le_bytes()
        })
        .flatten();
    bitmap.extend(colors);
    Some(bitmap)
}
#[test]
pub fn build_font() {
    let input = generate_font(include_bytes!(
        "/home/vik/Documents/MelonDS/emuSD/Light Theme/font.bmp"
    ))
    .expect("A");
    let mut a = vec![0u8; 2];
    a.extend_from_slice(&input);
    std::fs::write("/home/vik/Documents/MelonDS/emuSD/Light Theme/font.bin", a);
}
pub mod build_binaries;

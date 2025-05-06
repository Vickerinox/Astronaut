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
fn compress(data: &[u8]) -> Vec<u8> {
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

fn decomp_parse(data: &[u8]) -> Vec<Block> {
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
fn decompress(data: &[u8]) -> Vec<u8> {
    let signature = data[0];
    let size = u32::from_le_bytes([data[1], data[2], data[3], 0]) as usize;
    assert!(signature == 16);
    let decomp = decomp_parse(&data[4..]);

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
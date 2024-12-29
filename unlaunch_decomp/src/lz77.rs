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
    /*
    let mut output = vec![0u8; size];
    println!("sign: {signature} size: {size}");
    let mut src_pos = 4;
    let mut dst_pos = 0;
    while dst_pos < size {

        let mut flags = data[src_pos] as u16;
        src_pos += 1;

        //print!("flags: {flags:08b}, ");
        for _ in 0..8 {
            if dst_pos >= size {break};
            flags <<= 1;
            if flags & 0x100 == 0 {
                output[dst_pos] = data[src_pos];
                //print!("u{:02x}, ",data[src_pos]);
                dst_pos += 1;
                src_pos += 1;

            } else {
                let len = 3 + (data[src_pos]>>4);
                let disp = 1 + u16::from_be_bytes([data[src_pos] & 0xf, data[src_pos+1]]) as usize;
                src_pos += 2;
                //print!("c[{len:x},{disp:x}], ");
                for _ in 0..len {
                    output[dst_pos] = output[dst_pos-disp];
                    dst_pos += 1;
                }
            }
        }
        //println!("dst: {dst_pos}, src: {src_pos}");
    }
    output
    */
}
#[cfg(test)]
mod test {
    const FUZZING_AMOUNT: usize = 100_000;
    use std::fmt::Write as _;
    use std::fs::File;
    use std::io::BufRead;
    use std::io::BufReader;
    use std::io::Write;

    use super::*;
    #[allow(unused)]
    use console::Style;
    use indicatif::ParallelProgressIterator;
    use rayon::iter::IntoParallelIterator;
    use rayon::iter::ParallelBridge;
    use rayon::iter::ParallelIterator;
    #[allow(unused)]
    use similar::ChangeTag;
    #[allow(unused)]
    use similar::TextDiff;
    fn check_compress_decompress(vec: &Vec<u8>) -> bool {
        let compressed = compress(&vec);
        let data = decompress(&compressed);
        vec == &data
    }
    #[test]
    fn unlaunch_bin() {
        let input = include_bytes!("./unlaunch.bin");
        let data = decompress(input);
        let compress_again = compress(&data);
        let double_check = decompress(&compress_again);
        //std::fs::write("./unlaunchc.bin", &compress_again);
        assert_eq!(&data, &double_check);
        /*
        let dec_1 = decomp(&input[4..]);
        let dec_2 = decomp(&compress_again[4..]);
        println!("{} {}", dec_1.len(), dec_2.len());
        for (a,b) in dec_1.into_iter().zip(dec_2.into_iter()) {
            println!("{:02x?} vs {:02x?}", a,b);
        }

        */
    }
    #[test]
    fn first_bytes() {
        let input = include_bytes!("./unlaunch.bin").to_vec();
        let data = decompress(&input);
        let compress_again = compress(&data);
        let double_check = decompress(&compress_again);
        println!("{:?}\n{:?}", &input[0..128], &data[0..128]);
        assert_eq!(&data, &double_check);

        let was = input
            .into_iter()
            .take(128)
            .map(|e| hex::encode(&[e]))
            .collect::<Vec<_>>()
            .join(" ");
        let is = compress_again
            .into_iter()
            .take(128)
            .map(|e| hex::encode(&[e]))
            .collect::<Vec<_>>()
            .join(" ");

        println!("was:{}", was);
        println!("is:{}", is);
        let diff = TextDiff::from_words(&was, &is);
        let mut no_change = Vec::new();
        for change in diff.iter_all_changes() {
            let (sign, style) = match change.tag() {
                ChangeTag::Delete => ("-", Style::new().red()),
                ChangeTag::Insert => ("+", Style::new().green()),
                ChangeTag::Equal => {
                    no_change.push(change);
                    continue;
                }
            };
            for c in &no_change {
                eprint!(" {}", c.to_string().trim());
            }
            no_change.clear();
            eprint!(
                "{}{}",
                style.apply_to(sign).bold(),
                style.apply_to(change.to_string().trim())
            );
        }
    }
    use std::time::{Duration, Instant};
    #[test]
    fn fuzz_test() {
        let mut inputs: Vec<Vec<u8>> = Vec::new();
        // Load failed inputs from file
        let start = Instant::now();
        if let Ok(file) = File::open("failed_inputs.txt") {
            let reader = BufReader::new(file);
            for line in reader.lines() {
                if let Ok(line) = line {
                    let input: Vec<u8> = line
                        .split_whitespace()
                        .map(|byte_str| byte_str.parse().unwrap())
                        .collect();
                    inputs.push(hex::decode(input).expect("file corrupt"));
                }
            }
        }

        println!(
            "opening file: nanoseconds:{}",
            (Instant::now() - start).as_secs()
        );
        // Generate and test new random inputs
        let start = Instant::now();
        let new_inputs = (0..FUZZING_AMOUNT)
            .par_bridge()
            .map(|_| (0..2000).map(|_| rand::random()).collect());
        let failed = inputs
            .into_par_iter()
            .progress()
            .chain(new_inputs)
            .filter(|e| !check_compress_decompress(e));
        // Save all failed inputs to file after all tests
        let mut length = 0;
        if let Ok(mut file) = File::create("failed_inputs.txt") {
            if let Some((string, len)) = failed
                .fold_with((Vec::<u8>::new(), 0_u128), |(mut v, i), input| {
                    v.extend_from_slice((hex::encode(input) + "\n").as_bytes());
                    (v, i + 1)
                })
                .reduce_with(|(mut a, b), (c, d)| {
                    a.extend(c);
                    (a, b + d)
                })
            {
                file.write(&string).expect("Could not write to file");
                length = len;
            }
        }
        println!(
            "running nanoseconds:{}",
            (Instant::now() - start).as_secs()
        );
        println!(
            "{}% fuzzing attemps failed out of {}",
            (length as f64 / FUZZING_AMOUNT as f64) * 100_f64,
            FUZZING_AMOUNT
        );
        if length > 0 {
            panic!("Test failed for {} inputs", length);
        }
    }
}

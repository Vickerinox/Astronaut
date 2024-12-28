#[test]
fn unlaunch_bin() {
	let input = include_bytes!("./unlaunch.bin");
	let data = decompress(input);
	let compress_again = compress(&data);
	let double_check = decompress(&compress_again);
	assert_eq!(&data, &double_check);
	println!("{} {}", input.len(), compress_again.len());
}


pub enum Block {
	Uncompressed(u8),
	Compressed {
		disp: u16,
		len: u8,
	}
}

fn compress(data : &[u8]) -> Vec<u8> {
	let mut output = Vec::new();
	let mut pos = 0;
	while pos < data.len() {
		let (disp, len) = find_longest_match(data, pos);
		
		if len < 3 {
			output.push(Block::Uncompressed(data[pos]));
			pos += 1;
		} else {
			
			output.push(Block::Compressed { disp: disp as u16, len: len as u8 });
			pos = pos + (len as usize);
		}
	}
	let output: Vec<u8> = output.chunks(8).map(|i| {
		let mut output = vec![0u8];
		
		for block in i {
			output[0] <<= 1;
			match block {
				Block::Uncompressed(byte) => {
					output.push(*byte);
				},
				Block::Compressed { disp, len } => {
					output[0] |= 1;
					let len = len-3;
					let [hi, lo] = disp.to_be_bytes();
					let hi = (hi&0xF) | (len<<4);
					output.push(hi);
					output.push(lo);
				},
			}
		}
		output[0] <<= 8-i.len();
		output
	}).flatten().collect();
	let mut final_output = (((data.len() as u32) << 8) | 0x10).to_le_bytes().to_vec();
	final_output.extend_from_slice(&output);
	final_output
}

const MAX_DIST: usize = 0x1000;
const MAX_LEN: usize = 3+0xF;
// pos is current position in input value
fn find_longest_match(data : &[u8], pos : usize) -> (usize, usize) {
	let mut best_offset = 0;
	let mut best_len = 0;
	let start = if pos < MAX_DIST {
		0
	} else {
		pos - MAX_DIST
	};
	
	for offset in start..pos {
		let len = matching_len(data, offset, pos);
		if len > best_len {
			best_offset = pos - (offset as usize) - 1;
			best_len = len;
		}
	}
	return (best_offset, best_len);
}

fn matching_len(data : &[u8], mut offset : usize, mut pos : usize) -> usize {
	let mut len = 0;
	while pos < data.len() && data[offset] == data[pos] && len < MAX_LEN {
		offset += 1;
		pos += 1;
		len += 1;
	}
	return len;
}

fn decompress(data : &[u8]) -> Vec<u8> {

	let signature = data[0];
	assert!(signature == 16);
	let size = u32::from_le_bytes([data[1], data[2], data[3], 0]) as usize;
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
}
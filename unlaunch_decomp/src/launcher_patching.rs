
#[derive(Debug)]
pub struct UnlaunchBlockPatch {
    original_fn: Vec<u16>,
    patch_block_offset: i16,
    patch_block: Vec<u16>,
    skipped: bool,
}

#[test]
fn apply_patch() {
    let launcherd = &include_bytes!("/home/vik/Downloads/launcherd.nds")[0x4000..][..0xb5820];
    let mut l_words: Vec<_> = launcherd.chunks_exact(2).map(|i| u16::from_le_bytes([i[0], i[1]])).collect();
    let mut l_words2 = l_words.clone();
    let patch = include_bytes!("./launcher_patch.bin");
    let as_words = patch.chunks_exact(2).map(|i| u16::from_le_bytes([i[0], i[1]]));
    let patch = decode_patch(as_words);
    let (a,b,c) = translate_patch(&patch);
    let vlaunch_patch = VPatch { blocks: &a, originals: &b, patches: &c };
    println!("{vlaunch_patch:#04x?}");
    app_unlaunch_patch(&mut l_words, patch);
    let a = app_vlaunch_patch(&mut l_words2, &vlaunch_patch);
    println!("result: {a:?}");
    let mut expected_words: Vec<u16> = include_bytes!("/home/vik/Documents/NO$GBA/SLOT/patched_launcher.bin").chunks_exact(2).map(|i| u16::from_le_bytes([i[0], i[1]])).collect();
    for (i, (a,b)) in l_words.iter().zip(&expected_words).enumerate() {
        if *a != *b {
            println!("mismatch at offset {:x?} ({a:04x} vs {b:04x})", i*2);
        }
    }
    for (i, (a,b)) in l_words2.iter().zip(&expected_words).enumerate() {
        if *a != *b {
            println!("mismatch at offset {:x?} ({a:04x} vs {b:04x})", i*2);
        }
    }
    assert!(!vlaunch_patch.patches.contains(&0x980a))
}

#[derive(Debug)]
pub struct VPatch<'a> {
    blocks: &'a [VBlock],
    originals: &'a [u16],
    patches: &'a [u16],
}
#[derive(Debug)]
pub struct VBlock {
    original_len: u16,
    patch_len: u16,
    offset: i16,
}
#[derive(Debug)]
pub enum VPatchResult {
    Ok,
    BinaryRanOut,
    BadPatch,
    MatchRanOut,
    PatchRanOut,
    MalformedPatch,
}
fn app_vlaunch_patch(l_words: &mut [u16], patch: &VPatch) -> VPatchResult {
    let VPatch { blocks, mut originals, mut patches } = patch;
    let mut l_cursor = 0;
    for block in blocks.iter() {
        let Some((orig, remainder)) = originals.split_at_checked(block.original_len as usize) else {return VPatchResult::MatchRanOut};
        originals = remainder;
        let Some((patch, remainder)) = patches.split_at_checked(block.patch_len as usize) else { return VPatchResult::PatchRanOut};
        patches = remainder;

        loop {
            loop {
                let Some(word) = l_words.get(l_cursor) else {
                    return VPatchResult::BinaryRanOut;
                };
                let Some(word2) = orig.get(0) else {
                    return VPatchResult::BadPatch;
                };
                if word == word2 {
                    break;
                } else {
                    l_cursor += 1;
                }
            }
            let match_length = l_words[l_cursor..].iter().zip(orig.iter()).filter(|(a,b)| (**b == 0xAAAA) || (**b == **a)).count();
            if match_length == orig.len() {
                break;
            } else {
                l_cursor+=match_length
            }
        }
        let patch_cursor = l_cursor.wrapping_add_signed((block.offset as isize)/2);
        for (src, dst) in patch.iter().zip(&mut l_words[patch_cursor..]) {
            *dst = *src
        }
    }
    if originals.is_empty() && patches.is_empty() {
        VPatchResult::Ok
    } else {
        VPatchResult::MalformedPatch
    }
}
fn app_unlaunch_patch(l_words: &mut [u16], patch: Vec<UnlaunchBlockPatch>) {

    let mut l_cursor = 0;

    for (i, patch) in patch.iter().enumerate() {
        if patch.original_fn.is_empty() {
            println!("HIT END");
            break;
        }
        //if patch.skipped {
        //    continue;
        //}
        println!("patch {i} {}", patch.patch_block_offset);
        loop {
            while l_words[l_cursor] != patch.original_fn[0] {l_cursor += 1};
            let match_length = l_words[l_cursor..].iter().zip(patch.original_fn.iter()).filter(|(a,b)| (**b == 0xAAAA) || (**b == **a)).count();
            if match_length == patch.original_fn.len() {
                println!("match at offset {l_cursor:x?}");
                break;
            } else {
                l_cursor+=match_length
            }
        }
        let patch_cursor = l_cursor.wrapping_add_signed((patch.patch_block_offset as isize)/2);
        let range = {
            let orig_range = l_cursor..(l_cursor+patch.original_fn.len());
            let patch_range = patch_cursor..(patch_cursor+patch.patch_block.len());
            orig_range.start.min(patch_range.start)..orig_range.end.max(patch_range.end)
        };
        println!("before: {:04x?}", &l_words[range.clone()]);
        for (i, word) in patch.patch_block.iter().enumerate() {
            l_words[patch_cursor+i] = *word;
        }
        println!("after : {:04x?}", &l_words[range]);
        
    
    }
}

#[test] 
fn unwind_patch() {
    let patch = include_bytes!("./launcher_patch.bin");
    let mut as_words = patch.chunks_exact(2).map(|i| u16::from_le_bytes([i[0], i[1]]));
    decode_patch(as_words);
}
fn translate_patch(patches: &[UnlaunchBlockPatch]) -> (Vec<VBlock>, Vec<u16>, Vec<u16>) {
    let mut blocks = Vec::new();
    let mut originals = Vec::new();
    let mut patchs = Vec::new();
    for patch in patches {
        blocks.push(VBlock { original_len: patch.original_fn.len() as _, patch_len: patch.patch_block.len() as _, offset: patch.patch_block_offset });
        originals.extend_from_slice(&patch.original_fn);
        patchs.extend_from_slice(&patch.patch_block);
    }
    (blocks, originals, patchs)
}
fn decode_patch(mut as_words: impl Iterator<Item = u16>)  -> Vec<UnlaunchBlockPatch> {
    let mut block_vec: Vec<_> = Vec::new();
    loop {
        let mut match_len = as_words.next().unwrap();
        let mut match_off = as_words.next().unwrap();
        let mut patch = as_words.next().unwrap();

        if match_len == 0 {
            break;
        }
        let skipped = if match_len & 0x8000 > 0 {
            true
        } else {
            false
        };
        let match_len = match_len & !0x8000;
        let mut m = Vec::with_capacity(match_len as usize/2);
        for _ in 0..(match_len/2) {
            m.push(as_words.next().unwrap());
        }
        let mut p = Vec::with_capacity(patch as usize/2);
        for _ in 0..(patch/2) {
            p.push(as_words.next().unwrap());
        }
        let block = UnlaunchBlockPatch { original_fn: m, patch_block_offset: match_off as i16, patch_block: p, skipped};
        println!("{:x?}", block);
        block_vec.push(block);
    }
    block_vec
}

unsafe fn fun_023fe000() {
    let r1 = 0x2FFE000; //DSI header base
    let mut r0 = (0x2FFE1B4); //change access controls
    r0 |= 8; // add access to SD card 
    //str r0 to header
    let r3 = 0x2FFE000; // get header again
    let r0 = 0x23FE27c; // launcher patch address?
    let r1 = 0x2FFE028; // ARM9 address
    let r2 = 0x2FFE02C; //ARM9 size
    fun_0x23fe040(r0, r1, r2);
}
unsafe fn fun_0x23fe040(r0: i32, r1: i32, r2: i32) {
    let mut p_run = r0 as *mut (); // patch address?
    let mut b_run = r1 as *mut (); // binary location
    let mut b_siz = r2 as usize; // binary size

    //0x23fe050
    loop {
        let mut r4 = (p_run as *const u16).read(); // patch header? (string match size?)
        p_run = p_run.byte_add(2);
        let r5 = (p_run as *const i16).read(); // patch header? (patch offset from match?)
        p_run = p_run.byte_add(2);
        let mut patch_len = (p_run as *const u16).read(); // patch header? (patch size?)
        p_run = p_run.byte_add(2);

        if r4 == 0 {
            return
        }

        let first_word = (p_run as *const u16).read(); // patch header? (looks like first word in a match)
    
        //some sort of skip flag?
        let flag = r4 & 0x8000 == 0;
        r4 &= !0x8000;

    
        if !flag {
            let r0 = (0x02FF_FDF4 as *const u32).read() & 3 == 0;
            if !r0 {
                p_run = p_run.byte_add(r4 as usize);
                p_run = p_run.byte_add(patch_len as usize);
                continue;
            }
        }

        let _r9 = b_siz; //copy binary size???
        // find first word in a string match?
        loop {
            let mut r0 = (b_run as *const u16).read();
            b_run = b_run.byte_add(2);
            b_siz -= 2;
            if b_siz == 0 {
                //fun 23FE0EC, the failsafe!
                panic!();
            }
            if r0 != first_word {
                continue;
            }
            { // 0x23fe09c save r11, r10, r4
                let mut r11 = b_run;
                let mut r10 = p_run;
                let mut r4 = r4;
                let r3 = 0xAAAA; // some kind of wildcard???
                r11 = r11.byte_sub(2);
                while r4 != 0 {
                    let src1 = (r10 as *const u16).read();
                    let src2 = (r11 as *const u16).read();
                    if src1 != r3  {
                        if src1 != src2 {
                            break;
                        }
                    } 
                    r10 = r10.byte_add(2);
                    r11 = r11.byte_add(2);
                    r4 -= 2;
                }
                if r4 == 0 {
                    break;
                }
            }
            
        }
        b_run = b_run.byte_sub(2);
        p_run = p_run.byte_add(r4 as usize);
        let mut patch_block = b_run.byte_offset(r5 as isize);
        while patch_len != 0 {
            let r0 = (p_run as *const u16).read();
            p_run = p_run.byte_add(2);
            (patch_block as *mut u16).write(r0);
            patch_block = patch_block.byte_add(2);
            patch_len -= 2;
        }
        
    }
    
}
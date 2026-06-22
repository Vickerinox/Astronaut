
#[derive(Debug)]
pub struct UnlaunchBlockPatch {
    original_fn: Vec<u16>,
    patch_block_offset: i16,
    patch_block: Vec<u16>,
    skipped: bool,
}

#[test] 
fn unwind_patch() {
    let patch = include_bytes!("./launcher_patch.bin");
    let mut as_words = patch.chunks_exact(2).map(|i| u16::from_le_bytes([i[0], i[1]]));
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
        for words in 0..(match_len/2) {
            m.push(as_words.next().unwrap());
        }
        let mut p = Vec::with_capacity(patch as usize/2);
        for words in 0..(patch/2) {
            p.push(as_words.next().unwrap());
        }
        let block = UnlaunchBlockPatch { original_fn: m, patch_block_offset: match_off as i16, patch_block: p, skipped};
        println!("{:x?}", block);
        block_vec.push(block);
    }
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
const NTR_BLOWFISH_TABLE: &[u8] = &[];
#[test]
pub fn test_blowfish() {
    let mut app = vec![]; //include_bytes!("/home/vik/Documents/NocashGBA/SD card/photod.nds").to_vec();
    let (head, app) = app.split_at_mut(0x1000);
    let mut app_worded: Vec<_> = app
        .chunks_exact(4)
        .map(|i| u32::from_le_bytes([i[0], i[1], i[2], i[3]]))
        .collect();
    let mut temp = vec![0u64; head.len() / 8];
    unsafe {
        core::slice::from_raw_parts_mut(core::ptr::addr_of!(temp[0]) as *mut u8, 0x1000)
            .copy_from_slice(head);
    };
    let header = unsafe { &*(core::ptr::addr_of!(temp[0]) as *const common::bootstrap::HeaderTWL) };
    if !(0x4000..0x8000).contains(&header.head.arm9_offset) {
        println!("NO SECURE AREA!")
    }
    println!("test {:x?}", header.head.arm9_offset);
    let o = (header.head.arm9_offset - 0x1000) as usize;
    if app[o..][..8] == 0xE7FFDEFFE7FFDEFFu64.to_le_bytes() {
        println!("SECURE AREA ALREADY DECRYPTED!")
    }
    let mut bf = common::blowfish::BFCTX::new();

    let a = &mut app_worded[(o / 4)..];
    let mut tmp2 = a[1];
    let mut tmp1 = a[0];

    bf.init1(NTR_BLOWFISH_TABLE);

    let gamecode = header.head.tid;
    let mut arg = [gamecode, gamecode >> 1, gamecode << 1];
    bf.init2(&mut arg);
    println!("{:x?} {:x?} {:x?}", arg, tmp2, tmp1);
    bf.init2(&mut arg);
    bf.decrypt(&mut tmp2, &mut tmp1);
    println!("{:x?} {:x?} {:x?}", arg, tmp2, tmp1);
    arg[1] <<= 1;
    arg[2] >>= 1;
    bf.init2(&mut arg);
    bf.decrypt(&mut tmp2, &mut tmp1);
    println!("{:x?} {:x?} {:x?}", arg, tmp2, tmp1);
    println!("{:x?}", b"encryObj");

    /*
    let mut context = common::blowfish::BlowfishContext::new();
    let mut app = include_bytes!("/home/vik/Documents/NocashGBA/SD card/photod.nds").to_vec();
    let a = core::ptr::addr_of!(context) as *mut u8;
    for (i, byte) in NTR_BLOWFISH_TABLE.iter().enumerate() {
       unsafe{ a.add(i).write(*byte);    }
    }
    let (head, app) = app.split_at_mut(0x1000);
    let mut temp = vec![0u64; head.len()/8];
    unsafe { core::slice::from_raw_parts_mut(core::ptr::addr_of!(temp[0]) as *mut u8, 0x1000).copy_from_slice(head);};
    let header = unsafe { &*(core::ptr::addr_of!(temp[0]) as *const common::bootstrap::HeaderTWL)};
    if !(0x4000..0x8000).contains(&header.arm9_offset) {
        println!("NO SECURE AREA!")
    }
    println!("test {:x?}", header.arm9_offset);
    let o = (header.arm9_offset-0x1000) as usize;
    if app[o..][..8] == 0xE7FFDEFFE7FFDEFFu64.to_le_bytes() {
        println!("SECURE AREA ALREADY DECRYPTED!")
    }
    let crc = crc16(0xffff, &app[o..][..(0x8000-header.arm9_offset as usize)]);
    println!("{crc} {}", header.secure_area_crc);
    println!("{:x?}", b"encryObj");
    context.transform_key1(header.tid, 3, 8);
    context.decrypt_buf(&mut app[o..][..(0x4800-header.arm9_offset as usize)]);
    let len = (&app[o..][..(0x4800-header.arm9_offset as usize)]).len();
    println!("{:x?} {:x?}", &mut app[o..][..(0x4800-header.arm9_offset as usize)], len)
    */
}

pub fn crc16(mut value: u16, buffer: &[u8]) -> u16 {
    /*
    val[0..7] = C0C1h,C181h,C301h,C601h,CC01h,D801h,F001h,A001h
    for i=start to end
        crc=crc xor byte[i]
        for j=0 to 7
        crc=crc shr 1:if carry then crc=crc xor (val[j] shl (7-j))
        next j
    next i
    */
    let vals = [
        0xC0C1, 0xC181, 0xC301, 0xC601, 0xCC01, 0xD801, 0xF001, 0xA001,
    ];
    for byte in buffer {
        value ^= *byte as u16;
        for i in 0..8 {
            value >>= 1;
            if value & 0x1 != 0 {
                value = value ^ (vals[i] << (7 - i))
            };
        }
    }
    value
}

use reboot_lib::VIDEO_HARDWARE;

pub unsafe fn flush_mmc() {
    #[cfg(target_arch = "arm")]
    core::arch::asm!(
        "MCR p15, 0, r0, c7, c10, 4", //drain write buffer
        in("r0") 0,
    );
    for i in 0..4 {
        for j in 0..0x20 {
            let arg = (i << 30) | (j << 5);
            #[cfg(target_arch = "arm")]
            core::arch::asm!(
                "MCR p15, 0, r0, c7, c10, 2", //clean dcache entry
                in("r0") arg,
            );
        }
    }
    #[cfg(target_arch = "arm")]
    core::arch::asm!(
        "MCR p15, 0, r0, c7, c10, 4", //drain write buffer
        "MCR p15, 0, r0, c7, c5, 0", //Flush ICache
        "MCR p15, 0, r0, c7, c6, 0", //Flush DCache
        in("r0") 0,
    );
}

unsafe fn mysterious_function_2() {
    //WRAM C set to appear on arm9,
    core::ptr::write_volatile(0x400405c as *mut u32, 0x0C003800);
    core::ptr::write_volatile(0x4004050 as *mut u8, 0x80);

    //mmc flush
    flush_mmc();
    let mut r1 = 0x3800000;
    let mut r2 = 0x8000;

    //fill 32KB with our entry address?
    while r2 != 0 {
        core::ptr::write_volatile(r1 as *mut u32, 0x6023CD8);
        r1 += 4;
        r2 -= 4;
    }
    //mmc flush
    flush_mmc();

    //Remap WRAM C Back.
    let r0 = core::ptr::read_volatile(0x4004050 as *const u32);
    let r0 = r0 & 0xFF00FF00 | 0x99;
    core::ptr::write_volatile(0x4004050 as *mut u32, r0);
}

pub unsafe fn mysterious_takeover_function() {

    core::ptr::write_volatile(0x4000243 as *mut u8, 0x80);
    flush_mmc();

    //remember this is where the wram appears on the arm9
    //when we map this back it will appear at  0x37F0000..=0x37F7FFF
    const MAGIC_JUMP_START: *mut u32 = 0x3803040 as *mut u32;
    const BINARY_ENTRY_ADDR_ARM9: *mut u32 = 0x6860000 as *mut u32;
    const BINARY_ENTRY_ADDR_ARM7: u32 = 0x6000000;

    let mut arm7_bytes = crate::ARM7_BINARY.iter().copied();
    for i in 0..0x4000 {
        let byte1 = arm7_bytes.next().unwrap_or(0);
        let byte2 = arm7_bytes.next().unwrap_or(0);
        let byte3 = arm7_bytes.next().unwrap_or(0);
        let byte4 = arm7_bytes.next().unwrap_or(0);
        let stuff = u32::from_le_bytes([byte1, byte2, byte3, byte4]);
        BINARY_ENTRY_ADDR_ARM9.add(i).write_volatile(stuff);
    }

    //WRAM C set to appear on arm9,
    core::ptr::write_volatile(0x400405c as *mut u32, 0x0C003800);
    core::ptr::write_volatile(0x4004050 as *mut u8, 0x80);

    flush_mmc();

    for i in 0..0x800 {
        MAGIC_JUMP_START
            .add(i)
            .write_volatile(BINARY_ENTRY_ADDR_ARM7);
    }

    flush_mmc();

    core::ptr::write_volatile(0x4000243 as *mut u8, 0x82); //set VRAM D to arm7
    //Remap WRAM C Back to arm7.
    let r0 = core::ptr::read_volatile(0x4004050 as *const u32);
    let r0 = r0 & 0xFF00FF00 | 0x99;
    core::ptr::write_volatile(0x4004050 as *mut u32, r0);
}

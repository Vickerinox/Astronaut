//copies and sets up v/w ram
unsafe fn mysterious_function_1() {
    nocash_memcopy(0x6020000 as *mut u8, 0x6860000 as *mut u8, 0x20000); //memcpy binary from VRAM A to VRAM D
    flush_mmc(); //mpu reset? see below
    //Remaps VRAM D,C,B,A
    core::ptr::write_volatile(0x0400_0240 as *mut u32, 0x8A848981);
}

pub unsafe fn flush_mmc() {
    core::arch::asm!(
        "MCR p15, 0, r0, c7, c10, 4",
        in("r0") 0,
    );
    let mut arg = 0_u32;
    let mut carry = false;
    while !carry {
        while arg & 0x400 == 0 {
            core::arch::asm!(
                "MCR p15, 0, r0, c7, c10, 2",
                inout("r0") arg,
            );
            arg += 0x20;
        }
        (arg, carry) = (arg & !0x400).overflowing_add(0x40000000);
    }

    core::arch::asm!(
        "MCR p15, 0, r0, c7, c10, 4",
        "MCR p15, 0, r0, c7, c5, 0",
        "MCR p15, 0, r0, c7, c6, 0",
        in("r0") 0,
    );
}

unsafe fn nocash_memcopy(r0: *mut u8, r1: * mut u8, size: usize) {
    core::ptr::copy(r0, r1, size);
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

pub unsafe fn our_mysterious_function() {
    //WRAM C set to appear on arm9, 
    core::ptr::write_volatile(0x400405c as *mut u32, 0x0C003800);
    core::ptr::write_volatile(0x4004050 as *mut u8, 0x80);

    //mmc flush
    flush_mmc();
    let mut r1 = 0x3803000;
    let mut r2 = 0x1000;

    //fill 32KB with our entry address?
    while r2 != 0 {
        core::ptr::write_volatile(r1 as *mut u32, 0xEC);
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
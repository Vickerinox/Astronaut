pub struct SHA1State([u32; 25]);
#[allow(unused_variables)]
pub unsafe fn swi_sha1_calc(dest: *mut u8, source: *const u8, len: usize) {
    #[cfg(target_arch = "arm")] //MADDERFAKING BITHC RUST ANALYSER
    reboot_lib::critical_function(|| {
        core::arch::asm!(
            "SWI 0x270000",
            in("r0") dest,
            in("r1") source,
            in("r2") len,
            lateout("r0") _,
            lateout("r1") _,
            lateout("r2") _,
            lateout("r3") _,
        );
    });
}

#[allow(unused_variables)]
pub unsafe fn swi_crc16(start: u16, source: *const u16, len: usize) -> u16 {
    
    let mut retu = start;
    #[cfg(target_arch = "arm")] //MADDERFAKING BITHC RUST ANALYSER
    reboot_lib::critical_function(|| {
        core::arch::asm!(
            "SWI 0xE0000",
            in("r0") start,
            in("r1") source,
            in("r2") len,
            lateout("r0") retu,
            lateout("r1") _,
            lateout("r2") _,
            lateout("r3") _,
        );
    });
    retu
}



pub unsafe fn generate_cid_key(buf: &mut [u32; 4]) {
    swi_sha1_calc(buf as *mut u32 as *mut _, 0x2FFD7BC as *const u8, 0x10);
}

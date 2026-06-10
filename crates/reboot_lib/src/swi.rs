pub unsafe fn swi_delay(duration: u32) {
    #[cfg(target_arch = "arm")] //MADDERFAKING BITHC RUST ANALYSER
    crate::critical_function(|| {
        core::arch::asm!(
            "SWI 0x30000",
            in("r0") duration,
            lateout("r0") _,
            out("r1") _,
            out("r2") _,
            out("r3") _,
        );
    });
}

pub struct SHA1State([u32; 25]);
#[allow(unused_variables)]
pub unsafe fn swi_sha1_calc(dest: *mut u8, source: *const u8, len: usize) {
    #[cfg(target_arch = "arm")] //MADDERFAKING BITHC RUST ANALYSER
    crate::critical_function(|| {
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
pub unsafe fn swi_crc16(start: u16, source: *const (), len: usize) -> u16 {
    
    let mut retu = start;
    #[cfg(target_arch = "arm")] //MADDERFAKING BITHC RUST ANALYSER
    crate::critical_function(|| {
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
pub unsafe fn swi_vblank() {
    #[cfg(target_arch = "arm")] //MADDERFAKING BITHC RUST ANALYSER
    crate::critical_function(|| {
        core::arch::asm!("SWI 0x50000");
    });
}
pub unsafe fn swi_halt() {
    #[cfg(target_arch = "arm")] //MADDERFAKING BITHC RUST ANALYSER
    core::arch::asm!("push {{r0-r3}}", "SWI 0x60000", "pop {{r0-r3}}",);
}

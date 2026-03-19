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

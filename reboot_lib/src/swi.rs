pub unsafe fn swi_delay(duration: u32) {
    #[cfg(target_arch = "arm")] //MADDERFAKING BITHC RUST ANALYSER
    crate::critical_function(|| {
        core::arch::asm!(
            "SWI 0x30000",
            in("r0") duration,
        );
    });
}

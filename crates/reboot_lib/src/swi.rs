// SPDX-FileCopyrightText: 2026 Viktor Karlsson <viktor@koda.re>
// SPDX-License-Identifier: MIT
#[cfg(any(feature = "arm9", feature = "arm7"))]
pub unsafe fn swi_delay(duration: u32) {
    #[cfg(target_arch = "arm")] //rust analyzer gets pissed if you remove this
    crate::critical_function(
        #[instruction_set(arm::t32)]
        || {
            core::arch::asm!(
                "SWI 0x3",
                in("r0") duration,
                lateout("r0") _,
                out("r1") _,
                out("r2") _,
                out("r3") _,
            );
        },
    );
}


#[cfg(any(feature = "arm9i", feature = "arm7i"))]
#[allow(unused_variables)]
pub unsafe fn swi_sha1_calc(dest: *mut u8, source: *const u8, len: usize) {
    #[cfg(target_arch = "arm")] //rust analyzer gets pissed if you remove this
    crate::critical_function(
        #[instruction_set(arm::t32)]
        || {
            core::arch::asm!(
                "SWI 0x27",
                in("r0") dest,
                in("r1") source,
                in("r2") len,
                lateout("r0") _,
                lateout("r1") _,
                lateout("r2") _,
                lateout("r3") _,
            );
        },
    );
}


#[cfg(any(feature = "arm9", feature = "arm7"))]
#[allow(unused_variables)]
pub unsafe fn swi_crc16(start: u16, source: *const (), len: usize) -> u16 {
    let mut retu = start;
    #[cfg(target_arch = "arm")] //rust analyzer gets pissed if you remove this
    crate::critical_function(
        #[instruction_set(arm::t32)]
        || {
            core::arch::asm!(
                "SWI 0xE",
                in("r0") start,
                in("r1") source,
                in("r2") len,
                lateout("r0") retu,
                lateout("r1") _,
                lateout("r2") _,
                lateout("r3") _,
            );
        },
    );
    retu
}

#[cfg(any(feature = "arm9", feature = "arm7"))]
pub unsafe fn swi_vblank() {
    #[cfg(target_arch = "arm")] //rust analyzer gets pissed if you remove this
    crate::critical_function(
        #[instruction_set(arm::t32)]
        || {
            core::arch::asm!("SWI 0x5");
        },
    );
}


#[cfg(any(feature = "arm9", feature = "arm7"))]
pub unsafe fn swi_halt() {
    #[cfg(target_arch = "arm")] //rust analyzer gets pissed if you remove this
    #[instruction_set(arm::t32)]
    core::arch::asm!("push {{r0-r3}}", "SWI 0x6", "pop {{r0-r3}}",);
}

// SPDX-FileCopyrightText: 2026 Viktor Karlsson <viktor@koda.re>
// SPDX-License-Identifier: GPL-3.0-or-later

#[instruction_set(arm::t32)]
#[cfg(target_arch = "arm")]
pub unsafe fn load_default_font() {
    use crate::resources::FONT_FILE;
    core::arch::asm!(
        "SWI 0x11",
        in("r0") core::ptr::addr_of!(*FONT_FILE) as *const u8,
        in("r1") 0x2FF1000,
        lateout("r0") _,
        lateout("r1") _,
        out("r2") _,
        out("r3") _,
    );
} 
#[cfg(not(target_arch = "arm"))]
pub unsafe fn load_default_font() {
    transfer_font_to_vram(&*(0x2FF1000 as *const _));
    panic!()
}

#[instruction_set(arm::a32)]
#[cfg(target_arch = "arm")]
pub unsafe fn init_font() {
    transfer_font_to_vram(&*(0x2FF1000 as *const _));
}
#[cfg(not(target_arch = "arm"))]
pub unsafe fn init_font() {
    panic!()
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BinFont {
    font_data: [u32; 0x200],
    colors: [u32; 4],
}
unsafe fn transfer_font_to_vram(font: &BinFont) {
    let BinFont { font_data, colors } = font;
    for (i, val) in font_data.iter().copied().enumerate() {
        (0x6818000 as *mut u32).add(i).write(val);
    }
    for (i, val) in colors.iter().copied().enumerate() {
        (0x06890000 as *mut u32).add(i).write(val);
    }
}

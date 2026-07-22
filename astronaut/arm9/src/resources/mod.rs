// SPDX-FileCopyrightText: 2026 Viktor Karlsson <viktor@koda.re>
// SPDX-License-Identifier: GPL-3.0-or-later

// cheaty macro to word align byte arrays
pub struct WordAligned<Bytes: ?Sized> {
    pub _align: [u32; 0],
    pub bytes: Bytes,
}
macro_rules! include_bytes_word_align {
    ($path:expr) => {{
        static ALIGNED: &WordAligned<[u8]> = &WordAligned {
            _align: [],
            bytes: *include_bytes!($path),
        };

        &ALIGNED.bytes
    }};
}

pub const FONT_FILE: &[u8] =
    include_bytes_word_align!(concat!(env!("OUT_DIR"), "/font_compressed.bin"));
reboot_lib::const_assert!(FONT_FILE.len() < 0x800, "Failed to compress Font!");
reboot_lib::const_assert!(
    FONT_FILE.len() > 0,
    "Please build the default Font before building the ARM9 Binary"
);

pub const ARM7_BINARY: &[u8] = include_bytes_word_align!(concat!(env!("OUT_DIR"), "/arm7.bin"));
reboot_lib::const_assert!(
    ARM7_BINARY.len() > 0,
    "Please build the ARM7 binary before building the ARM9 Binary"
);

pub const BOOTSTRAP_BINARY: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/bootstrap.bin"));
reboot_lib::const_assert!(
    BOOTSTRAP_BINARY.len() > 0,
    "Please build the Bootstrap binary before building the ARM9 Binary"
);

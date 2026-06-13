pub unsafe fn generate_cid_key(buf: &mut [u32; 4]) {
    crate::swi_sha1_calc(buf as *mut u32 as *mut _, 0x2FFD7BC as *const u8, 0x10);
}

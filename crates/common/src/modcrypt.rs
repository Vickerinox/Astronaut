use crate::bootstrap::HeaderTWL;

pub unsafe fn decrypt_arm9i(header: &HeaderTWL) -> Result<(), ()> {
    if header.head.twl_flags & (1 << 1) == 0 {
        return Ok(());
    }

    Ok(())
}
pub unsafe fn decypy_arm7i(header: &HeaderTWL) -> Result<(), ()> {
    if header.head.twl_flags & (1 << 1) == 0 || header.modcrypt2_len == 0 {
        return Ok(());
    }
    Ok(())
}

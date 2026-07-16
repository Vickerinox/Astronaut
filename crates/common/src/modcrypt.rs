// SPDX-FileCopyrightText: 2026 Viktor Karlsson <viktor@koda.re>
// SPDX-License-Identifier: MIT

use crate::bootstrap::TWLHeader;

pub unsafe fn decrypt_arm9i(header: &TWLHeader) -> Result<(), ()> {
    if header.head.twl_flags & (1 << 1) == 0 {
        return Ok(());
    }

    Ok(())
}
pub unsafe fn decrypt_arm7i(header: &TWLHeader) -> Result<(), ()> {
    if header.head.twl_flags & (1 << 1) == 0 || header.modcrypt2_len == 0 {
        return Ok(());
    }
    Ok(())
}

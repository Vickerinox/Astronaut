// SPDX-FileCopyrightText: 2026 Viktor Karlsson <viktor@koda.re>
// SPDX-License-Identifier: MIT

use crate::MemoryWrapper;
pub struct GlobalMBKs([volatile_register::RW<u32>; 5]);

#[cfg(any(feature = "arm7i", feature = "arm9i"))]
pub const GLOBAL_MBKS: MemoryWrapper<GlobalMBKs> = MemoryWrapper(0x4004040 as *mut GlobalMBKs);

#[cfg(any(feature = "arm7i", feature = "arm9i"))]
pub struct LocalMBKs([volatile_register::RW<u32>; 3]);
pub const LOCAL_MBKS: MemoryWrapper<LocalMBKs> = MemoryWrapper(0x4004054 as *mut LocalMBKs);

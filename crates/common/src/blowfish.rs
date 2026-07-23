// SPDX-FileCopyrightText: 2026 Viktor Karlsson <viktor@koda.re>
// SPDX-License-Identifier: MIT

use crate::bootstrap::TWLHeader;

#[derive(Clone)]
pub struct BFCTX {
    magic: [u32; 0x412],
}
impl BFCTX {
    fn lookup(&self, v: u32) -> u32 {
        let mut a = (v >> 24) & 0xFF;
        let mut b = (v >> 16) & 0xFF;
        let mut c = (v >> 8) & 0xFF;
        let mut d = (v >> 0) & 0xFF;

        a = self.magic[a as usize + 18 + 0];
        b = self.magic[b as usize + 18 + 256];
        c = self.magic[c as usize + 18 + 512];
        d = self.magic[d as usize + 18 + 768];

        d.wrapping_add(c ^ b.wrapping_add(a))
    }
    pub fn encrypt(&self, l: &mut u32, h: &mut u32) {
        let mut a = *l;
        let mut b = *h;
        for i in 0..16 {
            let c = self.magic[i] ^ a;
            a = b ^ self.lookup(c);
            b = c;
        }
        *h = a ^ self.magic[16];
        *l = b ^ self.magic[17];
    }
    pub fn decrypt(&self, l: &mut u32, h: &mut u32) {
        let mut a = *l;
        let mut b = *h;
        for i in (2..=17).rev() {
            let c = self.magic[i] ^ a;
            a = b ^ self.lookup(c);
            b = c;
        }
        *l = b ^ self.magic[0];
        *h = a ^ self.magic[1];
    }
    fn update(&mut self, arg: &mut [u8; 8]) {
        for j in 0..18 {
            let mut r3 = 0u32;
            for i in 0..4 {
                r3 <<= 8;
                r3 |= arg[(j * 4 + i) & 7] as u32;
            }
            self.magic[j] ^= r3;
        }

        let mut tmp1 = 0u32;
        let mut tmp2 = 0u32;
        for i in (0..18).step_by(2) {
            self.encrypt(&mut tmp1, &mut tmp2);
            self.magic[i + 0] = tmp1;
            self.magic[i + 1] = tmp2;
        }

        for i in (0..0x400).step_by(2) {
            self.encrypt(&mut tmp1, &mut tmp2);
            self.magic[i + 18 + 0] = tmp1;
            self.magic[i + 18 + 1] = tmp2;
        }
    }
    pub fn init2(&mut self, a: &mut [u32; 3]) {
        let [a2, b2, c2] = a;
        self.encrypt(c2, b2);
        self.encrypt(b2, a2);
        let [a, b, c, d] = a2.to_le_bytes();
        let [e, f, g, h] = b2.to_le_bytes();
        let mut arg = [a, b, c, d, e, f, g, h];
        self.update(&mut arg);
    }
    pub fn init1(&mut self, table: &[u8]) {
        for (i, chunk) in table.chunks_exact(4).enumerate() {
            self.magic[i] = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        }
    }
    pub fn new() -> Self {
        Self { magic: [0; _] }
    }
}
pub unsafe fn decrypt_secure_area(header: &TWLHeader) {
    if (0x4000..0x8000).contains(&header.head.arm9_offset) {
        if core::slice::from_raw_parts(header.head.arm9_load as *mut u32, 2) != &[0xE7FFDEFF; 2] {}
    }
}

use crate::bootstrap::HeaderTWL;
/* 
#[derive(Clone)]
#[repr(C)]
pub struct BlowfishContext {
    pub table: [u32; 18],
    sboxes: [[u32; 256]; 4],
}
impl BlowfishContext {
    pub fn new() -> Self {
        Self { table: [0; _], sboxes: [[0; _]; _] }
    }
    pub unsafe fn load_ntr_keys_from_bootloader(&mut self) {
        // WARNING: This is only valid if the blowfish initialization table is untouched from the bootrom
        // WARNING: This value is kept in TCM, meaning it may as well point to open bus if not checked
        // WARNING: This is genuinely a terrible idea most of the time, it's only valid use is singular.
        let bootloader_copy = 0x1FFC894 as *const BlowfishContext;
        *self = (*bootloader_copy).clone();
    }
    pub fn encrypt_buf(&mut self, buffer: &mut [u8]) {
        for chunk in buffer.chunks_exact_mut(8) {
            let Ok(chunker) = chunk.try_into() else {continue};
            let val = u64::from_le_bytes(chunker);
            chunk.copy_from_slice(&self.encrypt_value(val).to_le_bytes());
        }
    }
    pub fn encrypt_value(&mut self, value: u64) -> u64 {
        let mut y = value as u32;
        let mut x = (value >> 32) as u32;
        for i in 0..16 {
            let z = (self.table[i] ^ x).to_le_bytes();
            let a = self.sboxes[0][z[3] as usize];
            let b = self.sboxes[1][z[2] as usize];
            let c = self.sboxes[2][z[1] as usize];
            let d = self.sboxes[3][z[0] as usize];
            x = d.wrapping_add(c ^ (b.wrapping_add(a))) ^ y;
            y = u32::from_le_bytes(z);
        }
        (x ^ self.table[16]) as u64 | (((y ^ self.table[17]) as u64) << 32)
    }
    pub fn decrypt_buf(&mut self, buffer: &mut [u8]) {
        for chunk in buffer.chunks_exact_mut(8) {
            let Ok(chunker) = chunk.try_into() else {continue};
            let val = u64::from_le_bytes(chunker);
            chunk.copy_from_slice(&self.decrypt_value(val).to_le_bytes());
        }
    }
    pub fn decrypt_value(&mut self, value: u64) -> u64 {
        let mut y = value as u32;
        let mut x = (value >> 32) as u32;
        for i in (2..=17).rev() {
            let z = (self.table[i] ^ x).to_le_bytes();
            let a = self.sboxes[0][z[3] as usize];
            let b = self.sboxes[1][z[2] as usize];
            let c = self.sboxes[2][z[1] as usize];
            let d = self.sboxes[3][z[0] as usize];
            x = d.wrapping_add(c ^ (b.wrapping_add(a))) ^ y;
            y = u32::from_le_bytes(z);
        }
        (x ^ self.table[1]) as u64 | (((y ^ self.table[0]) as u64) << 32)
    
    }
    pub fn transform_key1(&mut self, code: u32, level: u32, modulo: u32) {
        let mut ccode = [0u8; 12];
        ccode[0..4].copy_from_slice(&code.to_le_bytes());
        ccode[4..8].copy_from_slice(&(code >> 1).to_le_bytes());
        ccode[8..12].copy_from_slice(&(code << 1).to_le_bytes());
        if level >= 1 {
            self.apply_key_code(&mut ccode, modulo);
        }
        if level >= 2 {
            self.apply_key_code(&mut ccode, modulo);
        }
        if level >= 3 {
            let mid = u32::from_le_bytes([ccode[4],ccode[5],ccode[6],ccode[7]]) << 1;
            ccode[4..8].copy_from_slice(&mid.to_le_bytes());
            let mid = u32::from_le_bytes([ccode[8],ccode[9],ccode[10],ccode[11]]) >> 1;
            ccode[8..12].copy_from_slice(&mid.to_le_bytes());
        
            self.apply_key_code(&mut ccode, modulo);
        }
    }
    fn apply_key_code(&mut self, code: &mut [u8; 12], modulo: u32) {
        self.encrypt_buf(&mut code[4..]);
        self.encrypt_buf(&mut code[..8]);

        let mut reversed_code = code.clone();
        reversed_code[0..4].reverse();
        reversed_code[4..8].reverse();
        reversed_code[8..12].reverse();
        let mut index = 0;

        for i in 0..self.table.len() {
            self.table[i] ^= u32::from_le_bytes([reversed_code[index],reversed_code[index+1],reversed_code[index+2],reversed_code[index+3]]);
            index += 1;
            if index == (modulo as usize >> 2) {
                index = 0;
            }
        }

        let mut scratch = 0u64;
        for i in 0..9 {
            scratch = self.encrypt_value(scratch);
            self.table[(i*2)+0] = scratch as u32;
            self.table[(i*2)+1] = (scratch >> 32) as u32;
        }
        for i in 0..4 {
            for j in 0..128 {
                scratch = self.encrypt_value(scratch);
                self.sboxes[i][(j*2)+0] = scratch as u32;
                self.sboxes[i][(j*2)+1] = (scratch >> 32) as u32;
            }
        }
    }
}
    */
    #[derive(Clone)]
pub struct BFCTX {
    magic: [u32; 0x412]
}
impl BFCTX {
    fn lookup(&self, v: u32) -> u32 {
        let mut a = (v >> 24) & 0xFF;
        let mut b = (v >> 16) & 0xFF;
        let mut c = (v >> 8) & 0xFF;
        let mut d = (v >> 0) & 0xFF;

        a = self.magic[a as usize +18+0];
        b = self.magic[b as usize +18+256];
        c = self.magic[c as usize +18+512];
        d = self.magic[d as usize +18+768];

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
                r3 |= arg[(j*4 + i) & 7] as u32;
            }
            self.magic[j] ^= r3;
        }

        let mut tmp1 = 0u32;
        let mut tmp2 = 0u32;
        for i in (0..18).step_by(2) {
            self.encrypt(&mut tmp1, &mut tmp2);
            self.magic[i+0] = tmp1;
            self.magic[i+1] = tmp2;
        }
        
        for i in (0..0x400).step_by(2) {
            self.encrypt(&mut tmp1, &mut tmp2);
            self.magic[i+18+0] = tmp1;
            self.magic[i+18+1] = tmp2;
        }
    }
    pub fn init2(&mut self, a: &mut [u32; 3]) {
        let [a2,b2,c2] = a;
        self.encrypt(c2,b2);
        self.encrypt(b2,a2);
        let [a,b,c,d] = a2.to_le_bytes();
        let [e,f,g,h] = b2.to_le_bytes();
        let mut arg = [a,b,c,d,e,f,g,h];
        self.update(&mut arg);
    }
    pub fn init1(&mut self, table: &[u8]) {
        for (i, chunk) in table.chunks_exact(4).enumerate() {
            self.magic[i] = u32::from_le_bytes([chunk[0],chunk[1],chunk[2],chunk[3]]);
        }
        
    }
    pub fn new() -> Self {
        Self {
            magic: [0; _]
        }
    }
}
pub unsafe fn decrypt_secure_area(header: &HeaderTWL) {
    if (0x4000..0x8000).contains(&header.head.arm9_offset) {
        if core::slice::from_raw_parts(header.head.arm9_load as *mut u32, 2) != &[0xE7FFDEFF; 2] {
            
        }
    }
}
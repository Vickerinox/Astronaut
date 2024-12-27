unsafe fn arm7_main() {
    //0x6023D04
    fun_0602bfc8();
    fun_0602bfdc();
    fun_060244b0();
    let r1 = 0x400_4020;
    let r0 = 1;
    (r1 as *mut u16).write_volatile(r0);
    let r1 = 0x400_0304;
    let r0 = (r1 as *const u16).read_volatile();
    let r0 = r0 | 2;
    (r1 as *mut u16).write_volatile(r0);
}
//init 0x400_0210?
unsafe fn fun_0602bfc8() {
    let r1 = 0x400_0000 as *mut u32;
    let r2 = (r1).byte_add(0x210).read_volatile();
    r1.byte_add(0x210).write_volatile(0);
}
//init 0x400_0218?
unsafe fn fun_0602bfdc() {
    let r1 = 0x400_0000 as *mut u32;
    let r2 = (r1).byte_add(0x218).read_volatile();
    r1.byte_add(0x218).write_volatile(0);
}
unsafe fn fun_060244b0() {
    let r0 = fun_0602441c();
    let r1 = 0x2FFFD8C as *mut u32;
    let r2 = (r1).read_volatile();
    let r0 = r0 | r2;
    r1.write_volatile(r0);
}
unsafe fn fun_0602441c() -> u32 {
    let r1 = 0x400_4000;
    let r0 = (r1 as *const u16).byte_offset(0x10).read_volatile();
    let r0 = r0 & 1;
    let r2 = (r0 as u32) << 0x1F;
    let r0 = (r1 as *const u32).byte_offset(0x81C).read_volatile();
    let r0 = r0 & 0x20;
    let r2 = (r0 | r2) << 0x19;
    let r1 = 0x603001C;
    let r0 = (r1 as *const u32).read_volatile();
    (r1 as *mut u32).write_volatile(r2);
    let r2 = r2 ^ r0;

    let r1 = 0x400_0000 as *mut u32;
    let r0 = r1.byte_add(0x130).read_volatile();
    let r0 = r0 << 0x16;
    let r0 = r0 >> 0x16;
    let r1 = r1.byte_add(0x134).read_volatile();
    //tst r1, 0x80_0000
    //orrne r2,r2,0x2000_0000
    let r1 = r1 & 0x30000;
    let r0 = r1 | r0;
    let r0 = !r0;
    let r1 = 0x303FF;
    let r0 = r1 & r0;
    let r0 = r0 | r2;
    r0
}
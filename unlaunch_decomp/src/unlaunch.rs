unsafe fn arm7_main() {
    //0x6023D04
    fun_0602bfc8(); // set 0x4000210 to 0 (disables interrupts)
    fun_0602bfdc(); // set 0x4000218 to 0 (disables interrupts)
    fun_060244b0();
    let r1 = 0x400_4020;
    let r0 = 1;
    (r1 as *mut u16).write_volatile(r0);
    let r1 = 0x400_0304;
    let r0 = (r1 as *const u16).read_volatile();
    let r0 = r0 | 2;
    (r1 as *mut u16).write_volatile(r0);
    let r1 = 0x400_0206;
    let r0 = 0x30;
    (r1 as *mut u16).write_volatile(r0);
    fun_06025c58();
    let r4 = 0x400_0000;
    let r0 = 0x6D00;
    (r4 as *mut u32).byte_add(0x180).write_volatile(r0);
    let mut r0 = 0x100;
    while r0 & 0xF != 0xD {
        r0 = 0x100;
        r0 = (r4 as *mut u32).byte_add(0x180).read_volatile();
    }
    //0x6023D64

    /*
    fun_602b738();

    fun_602c1a8();
    fun_6025bf4();

    fun_602bfb4();

    fun_602bfb4();
    fun_60255f0();

    fun_60253e0();

    fun_60254d0();
    fun_60244b0();

    //0x6023db0
    fun_06027568(); //dsi wifi init
    fun_060267a0(); //gpio init?

    fun_060244b0(); //controller read
    fun_06029cd8(); //sdmmc something something
    fun_060244b0(); //controller read
    fun_06024bc0(); //sdmmc something again
    fun_0602a5dc(); //something with main mem?
    //0x6023e30
    fun_0602b750(); // scfg mmc check
    fun_0602b760(); // scfg mmc checks

    //0x6023ec0

    //0x6023F60
    fun_060253e0(); //ipc related things
    fun_6027128(); //changing wram
    //aes stuff is done here too (shouldn't we already have done that?)

    */
    //0x6024010
    //ndma stuff is set up
    //more controller reads (meant for dev consoles?)
    //scfg stuff

    //jump to 0x380f700?

    //0x60244F4 Main program loop?
}

unsafe fn fun_06026698() {
    let r1 = 0x400_0000;
    let r0 = 0x73;
    (0x400_0138 as *mut u8).write_volatile(r0);
    let mut r0 = 1;
    fun_0602668c(&mut r0);
    let r0 = 0x77;
}
unsafe fn fun_06027604(r1: &mut u32) {
    fun_06026698();
    let mut r0 = 0x4E;
    fun_060266d0(&mut r0, r1);
    let mut r0 = 0x80;
    fun_060266d0(&mut r0, r1);
    fun_060266bc();
    fun_06026698();
}
unsafe fn fun_060266bc() {
    let r1 = 0x400_0000;
    let r0 = 0x73;
    (0x400_0138 as *mut u8).write_volatile(r0);
}
//RTC related
unsafe fn fun_060266d0(r0: &mut u32, r1: &mut u32) {
    let r5 = 0x74;
    let r4 = 0x400_0000;
    let mut r2 = 8;
    let mut r3 = (*r0 & 0xFF);
    while r2 != 0 {
        r3 <<= 1;
        *r1 = r5 + *r1;
        (0x400_0138 as *mut u8).write_volatile(*r1 as u8);
        *r0 = 5;
        fun_0602668c(r0);
        *r1 = *r1 | r2;
        (0x400_0138 as *mut u8).write_volatile(*r1 as u8);
        *r0 = 5;
        fun_0602668c(r0);
        *r0 = (0x400_0138 as *mut u8).read_volatile() as u32;
        let r3 = (r3 | *r0) << 0x1F;
        r2 -= 1;
    }
    *r0 = r3 << 0x18;
}
unsafe fn fun_0602668c(r0: &mut u32) {
    *r0 <<= 3;
    //swi wait
}

//EDO:: POSSIBLE ENTRY POINT
unsafe fn fun_06025710() {}
//EDO:: POWER MANAGEMENT
unsafe fn fun_06026fc0() {}

//EDO:: sends i2c commands
unsafe fn fun_06026ff8() {}
unsafe fn fun_060277ec() {
    let mut r1 = 0x4000;
    let mut r0 = 0x100;
    fun_06027b48(&mut r1, &mut r0);
}
unsafe fn fun_06027b48(r1: &mut u32, r0: &mut u32) {
    let r8 = *r1;
    *r1 = 0x474;
    //fun_06027ca0();
    *r0 = 0x2900000;
    *r1 = 0x10000479;
    let mut r2 = 3;
    let r3 = r8 >> 8;
    (r3 as *mut u32).write_volatile(*r0);
    fun_06028018(r0, r1, &mut r2);
    *r0 = 0x2900000;
    *r1 = 0x10000478;
    let mut r2 = 1;
    let r3 = r8 & 0xff;
    (r3 as *mut u32).write_volatile(*r0);
    fun_06028018(r0, r1, &mut r2);
}
unsafe fn fun_06028018(r0: &mut u32, r1: &mut u32, r2: &mut u32) {
    let r11 = *r0;
    let r12 = *r2;
    let mut r4 = *r1 << 9;
    *r1 &= 0x78000000;
    r4 |= *r1;
    r4 |= 0x4000000;
    r4 |= 0x80000000;
    let r3 = !(*r2 | 0x200);
    r4 |= r3;
    //... stuff with dsi wifi
}
unsafe fn fun_06027810() {
    //fun_06029c90();
    //fun_06027b8c(); //calls CRC16 eventually?
}
unsafe fn fun_06027820() {
    //fun_06027e64();
}
unsafe fn fun_06027e64(r0: &mut u32) {
    fun_06027cb8(r0);
}
unsafe fn fun_06027cb8(r0: &mut u32) {
    let r1 = 0x450;
    while *r0 == 0 {
        fun_06027c7c();
    }
}
unsafe fn fun_06027c7c() {}
//seems like it's a full init of the dsi wifi.
//this is in no way accurate to the real asm, but just for a picture on what to analyze.
unsafe fn fun_06027568() {
    let mut r0 = 0x1FD;
    let mut r1 = 0x603_A8D0;
    let mut r2 = 1;
    fun_06025518(&mut r0, &mut r1, &mut r2); //does stuff with the SPI BUS
    fun_06027604(&mut r1); //does stuff with the RTC??

    fun_060276c4(); // does stuff with the DSi Wifi
    let r8 = r0;
    fun_060277ec(); // further dsi wifi things
    if r8 != 0 {
        fun_06027810(); //no clue, but does something related to CRC eventually
    }
    /*
    fun_06027820(); // EVEN MORE dswifi?????
    if r8 != 0 {
        fun_06027894(); // probably something similar to 06027810
    }
    fun_06027918(); // TODO: ACTUALLY CHECK THIS, seems like more wifi things, calls similar functions to the ones above.
    fun_06027984(); // more of the same, wifi...
    fun_060279bc(); // more wifi things, but this one is also mass copying stuff around, hmm.....


    let r0 = 0x60284e6;
    fun_06027aac(); // more copying memory around
    let r1 = 0x418;
    let r0 = 0;
    fun_06027ca0(); // more wifi things..........
    fun_06027a58(); // MOAR
    let r1 = 0x400;
    let r0 = 0x40001;
    fun_06027ca0(); //think we know what this is by now...
    fun_06027a70(); //you thought it, it is more wifi
    fun_06027a58(); //MORE WIFI
    let r1 = 0x400;
    let r0 = 0x40000;
    fun_06027ca0(); //same as above
    let r1 = 4;
    let r0 = 0;
    fun_06027f2c(); // :|
    */
}
unsafe fn fun_060276c4() {
    let r1 = 4;
    //fun_06027f20();
    let r9 = 0x400_4A00;
}

//also related to spi?????
unsafe fn fun_06025518(r0: &mut u32, r1: &mut u32, r2: &mut u32) {
    *r0 &= 0xFF000000;
    let mut r3 = *r0 | 0x03000000;
    fun_060254f8(r2, &mut r3, r0);
    while *r0 as u8 != 0 {
        fun_06025484(r2, r0);
        (*r1 as *mut u8).write_volatile(*r0 as u8);
        *r1 += 1;
    }
}
unsafe fn fun_060254f8(r2: &mut u32, r3: &mut u32, r0: &mut u32) {
    *r2 += 4;
    let mut r6 = 0x18;
    *r0 = *r3 << r6;
    while r6 > 0 {
        fun_06025484(r2, r0);
        r6 -= 8;
    }
}
//STUFF WITH SPI BUS!!! DOUBLE CHECK!!! (seems like a spi read???)
unsafe fn fun_06025484(r2: &mut u32, r0: &mut u32) {
    let mut r3 = 0x8100;
    let r4 = 0x400_0000;
    (r4 as *mut u8).byte_add(0x208).write_volatile(r4 as u8);
    if r2 != &1 {
        r3 = r3 | 0x800;
    }
    let mut r0 = (r3 | *r0) << 0x10;
    (r4 as *mut u32).byte_add(0x1C0).write_volatile(r0);
    while r0 & 0x80 == 0 {
        r0 = (r4 as *mut u32).byte_add(0x1C0).read_volatile();
    }
    if r2 == &1 {
        r0 = 3;
    }
    //*r0 = (r4 as *mut u8).byte_add(0x1C2).read_volatile();
    if r2 == &1 {
        (r4 as *mut u8).byte_add(0x208).write_volatile(*r2 as u8);
    }
    *r2 -= 1;
}

//ENABLE INTERRUPTS
unsafe fn fun_0602bfb4(arg: u32) {
    let r1 = 0x400_0210 as *mut u32;
    let r2 = r1.read_volatile();
    let r2 = r2 | arg;
    r1.write_volatile(r2);
}

//FIFO RELATED
unsafe fn fun_06025c58() {
    let r1 = 0x400_0000;
    let r0 = 0x8408;
    (r1 as *mut u32).byte_add(0x184).write_volatile(r0);
}
//init 0x400_0210? INTERRUPT DISABLE
unsafe fn fun_0602bfc8() {
    let r1 = 0x400_0000 as *mut u32;
    let r2 = (r1).byte_add(0x210).read_volatile();
    r1.byte_add(0x210).write_volatile(0);
}
//init 0x400_0218? POSSIBLY ALSO INTERRUPTS?
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
//DISABLE NEW INTERRUPTS, ACKNOWLEDGE SDMMC
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

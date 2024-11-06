#![no_main]
#![no_std]
/// This function steals control of the ARM7 CPU assuming it is running in the sync loop within the bootloader.
/// The way it does this is by stealing some unused WRAM, writing code it, and mapping it to the memory where
/// the ARM7 is executing (the small sync loop)
/// 
/// TODO: instead of using NOP instructions that leads the CPU down the bank until it hits code use a jump table.
pub unsafe fn steal_arm7() {

    //steal WRAM-C4 from the arm7, as it *should* be unused while it's sync-looping.
    core::ptr::write_volatile(0x4004050 as *mut u8, 0b10000000);

    //map the WRAM to our cpu, making it visible in 0x0300_0000
    core::ptr::write_volatile(0x400405C as *mut u32, 0x100000);

    //Write NOP instructions to the area which the arm7 will be occupying (WRAM C slot 7)
    for i in 0x400..0x1000 {
        core::ptr::write_volatile((0x3000000+(i<<2)) as *mut u32, 0xE1A00000);
    }
    //Write our binary (for now a branch instruction that jumps to itself, AKA infinite loop.)
    core::ptr::write_volatile((0x3000000+(0x1000<<2)) as *mut u32, 0xEAFFFFFE);
   
    //overwrite the WRAM bank the arm7 is currently executing in with ours
    core::ptr::write_volatile((0x4004050) as *mut u8, 0b10011101); //enable our hijacked one
    core::ptr::write_volatile((0x4004053) as *mut u8, 0);           //disable the old one (unneccesary maybe?)
    
}


/// Main
#[no_mangle]
pub fn _start() {
    unsafe
    {
        //enable the 2D engine A, with no backgrounds on.
        core::ptr::write_volatile(0x4000000 as *mut u32, 0b000000000000000010000000000000000);
        //set background color to brat green.
        core::ptr::write_volatile(0x5000000 as *mut u16, 0b0000111101010100);
        
        steal_arm7();
        loop {}
    }
}

//Really our code should NEVER panic, but we still need this.
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#![no_main]
#![no_std]
/// This function steals control of the ARM7 CPU assuming it is running in the sync loop within the bootloader.
/// The way it does this is by stealing some unused WRAM, writing code it, and mapping it to the memory where
/// the ARM7 is executing (the small sync loop)
pub unsafe fn steal_arm7() {

    //offsets in words (4 bytes) into the WRAM were about to steal
    //JT here means "jump table", offsets could likely be tweaked, but i've not bothered to tune them.
    const ENTRYPOINT_OFFSET: usize = 0x800;
    const JT_START: usize = 0x400;
    const JT_END: usize = ENTRYPOINT_OFFSET-1; //-1 here prevents some shenanigans, don't worry about it.
    const BRANCH_BASE: usize = ENTRYPOINT_OFFSET-2; //because ARM instructions are "special", this is correct.

    //some magic constants never hurt ;)
    const BLANK_BRANCH_INSTRUCTION: u32 = 0xEA000000;
    const STOLEN_WRAM: *mut u32 = 0x03000000 as *mut u32;
    
    //steal WRAM-C4 from the arm7, as it *should* be unused while it's working.
    core::ptr::write_volatile(0x4004050 as *mut u8, 0b10000000);
    //map the WRAM to our cpu (arm7), and make it visible in 0x0300_0000
    core::ptr::write_volatile(0x400405C as *mut u32, 0x100000);

    //Write a big series of branch instructions that makes the arm7 jump directly to our entrypoint later
    for i in JT_START..JT_END {
        core::ptr::write_volatile(STOLEN_WRAM.add(i), BLANK_BRANCH_INSTRUCTION | (BRANCH_BASE-i) as u32);
    }
    //Write our entrypoint (for now a branch instruction that jumps to itself, AKA infinite loop.)
    core::ptr::write_volatile(STOLEN_WRAM.add(ENTRYPOINT_OFFSET), 0xEAFFFFFE);
   
    //overwrite the WRAM bank the arm7 is currently executing in with ours
    core::ptr::write_volatile((0x4004050) as *mut u8, 0b10011101); //enable our hijacked one
    core::ptr::write_volatile((0x4004053) as *mut u8, 0);           //disable the old one (unneccesary maybe?)
    
    //congrats! Now the arm7 is stolen.
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
#[cfg(not(test))] //works to shut up rust-analyzer in vscode. It keeps thinking we still have std...
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

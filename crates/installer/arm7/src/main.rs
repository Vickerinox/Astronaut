#![feature(ptr_metadata)]
#![no_main]
#![no_std]

const DSI_WRAM_START: usize = 0x0380_0000;
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _start() {
    core::arch::asm!(
        //turn off interrupts via the IME register
        "mov r0, #0x04000000",
        "str r0, [r0, #0x208]",

        //load start of stack(s)
        "mov r0, #0x12",
        "msr cpsr, r0",
        "ldr sp, ={stack_irq}",

        "mov r0, #0x13",
        "msr cpsr, r0",
        "ldr sp, ={stack_svc}",

        "mov r0, #0x1F",
        "msr cpsr, r0",
        "ldr sp, ={stack_sys}",

        // Call the main function
        "bl {main}",

        // Halt the CPU after main returns (if it does)
        "2: b 2b", // Infinite loop

        stack_irq = const DSI_WRAM_START + 0x1000,
        stack_svc = const DSI_WRAM_START + 0x2000,
        stack_sys = const DSI_WRAM_START + 0x3000,

        main = sym main, // Link the `main` symbol
        options(noreturn) // No return possible from this function
    );
}
fn main() {
    reboot_lib::standard_arm7::main_arm7();
}

#[cfg(target_arch = "arm")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe {
        (0x400_0208 as *mut u32).write_volatile(0);
        reboot_lib::IPC_FIFO_HARDWARE.set_status(7);
    }
    loop {}
}

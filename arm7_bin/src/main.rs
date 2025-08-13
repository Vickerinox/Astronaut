#![feature(ptr_metadata)]
#![no_main]
#![no_std]
mod swi;
mod mmc;

use core::arch::asm;
use reboot_lib::{
    spi::{Control, PowerRegiser},
    IPC_FIFO_HARDWARE,
};
const DSI_WRAM_START: usize = 0x037B8000;
#[no_mangle]
pub unsafe extern "C" fn _start() {
    asm!(
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


static mut FRAME_COUNTER: u32 = 0;
fn vblank_interrupt() {
    unsafe {FRAME_COUNTER += 1};
}
/// A interrupt handler appropriate for the ds, courtesy of libnds
unsafe fn interrupt_handler() {
    // what you are about to see is probably the most unoxidized code i've ever written -vikrinox
    core::arch::asm!(
        // According to libnds, r0-r3, as well as r12 and lr are saved by the BIOS handler.
        "mov r12, {i_base}",
        "ldr r1, [r12, {i_e}]",
        "ldr r2, [r12, {i_f}]",
        "ands r1, r1, r2", //the interrupt bits to be serviced! (i.e IE & IF)
        "beq 3f", // MEANING: if there are "no interrupts" to be serviced, it's gotta be the aux ones!

            // Get the bit index for the "highest priority" IRQ
            // Manually counting zeroes
            "mov r0, #0",
            "cmp r1, #0x10000",
            "movcs r1, r1, lsr #16",
            "addcs r0, r0, #16",
            "tst r1, 0xFF00",
            "movne r1, r1, lsr #8",
            "addne r0, r0, #8",
            "tst r1, 0xF0",
            "movne r1, r1, lsr #4",
            "addne r0, r0, #4",
            "tst r1, 0xC",
            "movne r1, r1, lsr #2",
            "addne r0, r0, #2",
            "add r0, r0, r1, lsr #1",

            "mov r1, #1",
            "mov r1, r1, lsl r0", //create a "bitmask" of the IRQ

            // Clear the interrupt on the hardware side
            "str r1, [r12, {i_f}]",

            // Clear the interrupt on the bios side
            "ldr r2, ={bios_f}",
            "ldr r3, [r2]",
            "orr r3, r3, r1",
            "str r3, [r2]",

            // load irq table and jump to funciton pointer
            "ldr r3, ={irq_table}",
            "add r3, r0, lsl #2",

            "b 4f",

        //check AUX irq's
        "3:",
            "ldr r1, [r12, {i_ae}]",
            "ldr r2, [r12, {i_af}]",
            "ands r1, r1, r2", //the interrupt bits to be serviced! (i.e IE & IF)
            "moveq pc, lr", // EARLY RETURN: There are no IRQ's to service!

            // Get the bit index for the "highest priority" IRQ
            // Manually counting zeroes
            "mov r0, #0",
            "cmp r1, #0x10000",
            "movcs r1, r1, lsr #16",
            "addcs r0, r0, #16",
            "tst r1, 0xFF00",
            "movne r1, r1, lsr #8",
            "addne r0, r0, #8",
            "tst r1, 0xF0",
            "movne r1, r1, lsr #4",
            "addne r0, r0, #4",
            "tst r1, 0xC",
            "movne r1, r1, lsr #2",
            "addne r0, r0, #2",
            "add r0, r0, r1, lsr #1",
            
            "mov r1, #1",
            "mov r1, r1, lsl r0", //create a "bitmask" of the IRQ

            // Clear the interrupt on the hardware side
            "str r1, [r12, {i_af}]",

            // Clear the interrupt on the bios side
            "ldr r2, ={bios_af}",
            "ldr r3, [r2]",
            "orr r3, r3, r1",
            "str r3, [r2]",

            // load irq table and jump to funciton pointer
            "ldr r3, ={irq_table_aux}",
            "add r3, r0, lsl #2",
        //Dereference the interrupt function pointer
        "4:",
        "ldr r3, [r3]",
        "cmp r3, #0",
        "beq 2f", //EARLY RETURN: no interrupt handler installed
            //set IME = 0
            "ldr r1, [r12, {ime}]",
            "str r12, [r12, {ime}]", //HACK: IME only cares about bit 0, so this sets IME = 0

            //get into system mode
            "mrs r0, spsr",
            "push {{r0,r1,r12,lr}}", // {spsr, ime, i_base, irq_lr}
            "mrs r0, cpsr",
            "bic r1, r0, {user_clear}",
            "orr r1, r1, {user_set}",
            "msr cpsr, r1",

            //run the interrupt handler
            "push {{r0, lr}}", // NOTE: we push LR *again* since system mode has it's own lr.
            "adr lr, 5f",
            "bx r3",         //execute interrupt handler (the moment we've been waiting for!!!)
            "5: pop {{r0, lr}}",

            //Hop out of system mode
            "msr cpsr, r0",
            "pop {{r0,r1,r12,lr}}", // {spsr, ime, i_base, irq_lr}
            "msr spsr, r0",

            //Restore IME
            "str r1, [r12, {ime}]",
        //return
        "2: mov pc, lr",

        i_base = const 0x0400_0000, //register base
        i_e = const 0x210,  //interrupt enable register
        i_f = const 0x214,  //interrupt request register
        i_ae = const 0x218,  //interrupt enable register
        i_af = const 0x21C,  //interrupt request register
        bios_f = const 0x380FFF8,   //interrupt request regiser (BIOS)
        bios_af = const 0x380FFC0,   //interrupt request regiser (BIOS)
        irq_table = sym INTERRUPT_TABLE,
        irq_table_aux = sym INTERRUPT_TABLE_AUX,
        ime = const 0x208,  //master interrupt enable
        user_clear = const 0x80 | 0x40 | 0x1F, //disable IRQ/FIQ masking, clear mode bits
        user_set = const 0x1F,  //Set mode to "System"
    );
}
static mut INTERRUPT_TABLE: [*mut fn(); 32] = [core::ptr::null_mut(); 32];
static mut INTERRUPT_TABLE_AUX: [*mut fn(); 15] = [core::ptr::null_mut(); 15];

fn main() {
    unsafe {
        IPC_FIFO_HARDWARE.enable();

        core::ptr::write_volatile(0x4000210 as *mut u32, 0);
        (0x4000208 as *mut u32).write_volatile(0);
        reboot_lib::spi::touchscreen::init_tsc();
        reboot_lib::i2c::init();
        reboot_lib::spi::write_powerman(PowerRegiser::Control(Control::ENABLE_BACKLIGHTS));

        (0x400_0008 as *mut u32)
            .write_volatile((0x400_0008 as *const u32).read_volatile() | (1 << 17));
        (0x400_0004 as *mut u32)
            .write_volatile((0x400_0004 as *const u32).read_volatile() | (1 << 2));

        let mut key = [0u32; 4];
        swi::generate_cid_key(&mut key);
        reboot_lib::load_nand_key_x(0);
        reboot_lib::load_nand_key_y(0, &[0x0AB9DC76, 0xBD4DC4D3, 0x202DDD1D, 0xE1A00005]);
        reboot_lib::nand_crypt_init(0);

        let mut buffer: *mut [reboot_lib::StorageSector] =
            core::slice::from_raw_parts_mut(0x2FFFE00 as *mut reboot_lib::StorageSector, 1);

        reboot_lib::IPC_FIFO_HARDWARE.set_status(1);
        while reboot_lib::IPC_FIFO_HARDWARE.read_status() != 1 {}
        reboot_lib::IPC_FIFO_HARDWARE.set_status(0);

        loop {}
        
        irq_init();

        match mmc::init_all() {
            Ok(_) => IPC_FIFO_HARDWARE.send_raw_blocking(0),
            Err(err) => IPC_FIFO_HARDWARE.send_raw_blocking(err.bits()),
        }
        match mmc::read_mmc_sectors(buffer, 0) {
            Ok(_) => IPC_FIFO_HARDWARE.send_raw_blocking(0xB00BB00B),
            Err(err) => IPC_FIFO_HARDWARE.send_raw_blocking(err.bits()),
        }
        //IPC_FIFO_HARDWARE.send_raw_blocking(send);
        loop {}
        /*
        loop {
            while IPC_FIFO_HARDWARE.recv_fifo_empty() {}
            let mut response = 0;
            match IPC_FIFO_HARDWARE.recieve_raw_blocking() {
                1 => {
                    let Some([0]) = gather_args() else { response = 0x8000_0000; continue;};
                    let controls = !core::ptr::read_volatile(0x4000130 as *const u16);
                    let mut controls = reboot_lib::Buttons::from_bits_retain(controls);
                    //if !reboot_lib::spi::touchscreen::is_pen_down() {
                    //    controls ^= reboot_lib::Buttons::PEN_DOWN;
                    //}
                    response = controls.bits() as u32;
                }
                2 => {
                    let Some([ptr, len]) = gather_args() else { response = 0x8000_0000; continue;};
                    buffer = core::slice::from_raw_parts_mut(ptr as *mut _, len as usize);
                }
                3 => {
                    let Some([arg]) = gather_args() else { response = 0x8000_0000; continue;};
                    response = match mmc_read_decrypt(buffer, &key, arg) {
                        Ok(_) => 0,
                        Err(e) => 0x8000_0000 | e.bits(),
                    };
                }
                4 => {

                }
                5 => {
                    let Some([arg]) = gather_args() else { response = 0x8000_0000; continue;};
                    sd_read_sectors(buffer, arg);
                }
                6 => {
                    let Some([arg]) = gather_args() else { response = 0x8000_0000; continue;};
                    IPC_FIFO_HARDWARE.send_raw_blocking(0);
                    (*(arg as *mut () as *mut unsafe extern fn()))();
                }
                7 => {
                    let Some([arg]) = gather_args() else { response = 0x8000_0000; continue;};
                    firmware_read(buffer, arg);
                }
                _ => {response = 0x8000_0000},
            }
            IPC_FIFO_HARDWARE.send_raw_blocking(response);
        }
        */
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe {
        IPC_FIFO_HARDWARE.set_status(7);
    }
    loop {}
}

pub unsafe fn firmware_read(data: *mut [reboot_lib::StorageSector], offset: u32) {
    let (ptr, len) = data.to_raw_parts();
    let buffer = core::slice::from_raw_parts_mut(ptr as *mut u8, len << 9);
    reboot_lib::spi::SPI_HARDWARE.read_firmware(buffer, offset);
}
/// read and decrypt the given sectors from NAND using NDMA.
pub unsafe fn mmc_read_decrypt(
    data: *mut [reboot_lib::StorageSector],
    ctr_base: &[u32; 4],
    sector: u32,
) -> Result<(), reboot_lib::Status> {
    mmc::read_mmc_sectors(
        core::slice::from_raw_parts_mut(0x0380_0000 as *mut reboot_lib::StorageSector, 1),sector)?;

    fn add_on_key(key: &mut [u32; 4], add: u32) {
        let carry;
        let carry2;
        let carry3;
        (key[0], carry) = key[0].overflowing_add(add);
        (key[1], carry2) = key[1].overflowing_add(carry as u32);
        (key[2], carry3) = key[2].overflowing_add(carry2 as u32);
        key[3] = key[3].wrapping_add(carry3 as u32);
    }
    let mut key = ctr_base.clone();
    add_on_key(&mut key, sector << 5);
    let ptr = data as *mut ();
    let len = data.len();
    reboot_lib::AES_HARDWARE.ctr_crypt_block(
        0x0380_0000 as *mut _,
        ptr as *mut _,
        (len << 6) as u32,
        &key,
    );
    Ok(())
}

/// read from the SD card using NDMA.
pub unsafe fn sd_read_sectors(
    data: *mut [reboot_lib::StorageSector],
    sector: u32,
) -> Result<(), ()> {
    use reboot_lib::ndma::{Control, NDMA_HARDWARE};

    let a = reboot_lib::read_sectors(reboot_lib::DeviceSelect::SDCardSlot, sector, data);
    match a {
        Ok(_) => (),
        Err(_) => return Err(()),
    }
    //await for everything to finish
    NDMA_HARDWARE.await_channel(0);
    Ok(())
}

pub unsafe fn nocash_write(str: &str) {
    const NOCASH_OUT_CHR: *mut u8 = 0x4fffa1c as *mut u8;
    for byte in str.as_bytes() {
        NOCASH_OUT_CHR.write_volatile(*byte);
    }
}

unsafe fn gather_args<const N: usize>() -> Option<[u32; N]> {
    let mut array = [0u32; N];
    for data in array.iter_mut() {
        *data = IPC_FIFO_HARDWARE.recieve_raw_blocking();
    }
    Some(array)
}

unsafe fn irq_init() {
    use reboot_lib::INTERUPT_HARDWARE;
    INTERUPT_HARDWARE.master.write(0);
    INTERUPT_HARDWARE.enable.write(0);
    INTERUPT_HARDWARE.request.write(!0);
    INTERUPT_HARDWARE.enable2.write(0);
    INTERUPT_HARDWARE.request2.write(!0);
    (0x0380_FFFC as *mut unsafe fn()).write(interrupt_handler);
    INTERUPT_HARDWARE.master.write(1);
}

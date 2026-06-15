/// A interrupt handler appropriate for the ds, courtesy of libnds
#[cfg(target_arch = "arm")]
#[instruction_set(arm::a32)]
unsafe fn interrupt_handler_arm7() {
    // what you are about to see is probably the most unoxidized code i've ever written -vikrinox
    
    core::arch::asm!(
        // r0-r3, as well as r12 and lr (r14) are saved by the original BIOS IRQ handler (Viewable at 0x0000006C).
        "mov r12, {i_base}",
        "ldr r1, [r12, {i_e}]",
        "ldr r2, [r12, {i_f}]",
        "ands r1, r1, r2", //the interrupt bits to be serviced! (i.e IE & IF)
        "beq 3f", // MEANING: if there are "no interrupts" to be serviced, it's gotta be the aux ones!

            // Get the bit index for the "highest priority" IRQ
            // Manually counting zeroes, as there is no instruction to do so on armv4
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
            "push {{r0, r4-r12, lr}}", // NOTE: we push LR *again* since system mode has it's own lr.
            "adr lr, 5f",
            "bx r3",         //execute interrupt handler (the moment we've been waiting for!!!)
            "5: pop {{r0, r4-r12, lr}}",

            //Hop out of system mode
            "msr cpsr, r0",
            "pop {{r0,r1,r12,lr}}", // {spsr, ime, i_base, irq_lr}
            "msr spsr, r0",

            //Restore IME
            "str r1, [r12, {ime}]",

        //return
        "2:",

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
#[cfg(not(target_arch = "arm"))]
unsafe fn interrupt_handler_arm7() {panic!()}

static mut INTERRUPT_TABLE: [*mut fn(); 32] = [core::ptr::null_mut(); 32];
static mut INTERRUPT_TABLE_AUX: [*mut fn(); 15] = [core::ptr::null_mut(); 15];

pub unsafe fn init_interrupts() {
    use crate::INTERUPT_HARDWARE;
    INTERUPT_HARDWARE.master.write(0);
    INTERUPT_HARDWARE.enable.write(0);
    INTERUPT_HARDWARE.request.write(!0);
    INTERUPT_HARDWARE.enable2.write(0);
    INTERUPT_HARDWARE.request2.write(!0);
    (0x0380_FFFC as *mut unsafe fn()).write(interrupt_handler_arm7);
    INTERUPT_HARDWARE.master.write(1);
}

const AUX_INTERRUPT: u8 = 32;
const INTERRUPT_INDEX_MASK: u8 = (AUX_INTERRUPT - 1);

#[repr(u8)]
pub enum ARM7Interrupt {
    VBlank = 0,
    HBlank = 1,
    VCounterMatch = 2,
    Timer0 = 3,
    Timer1 = 4,
    Timer2 = 5,
    Timer3 = 6,
    RTC = 7,
    DMA0 = 8,
    DMA1 = 9,
    DMA2 = 10,
    DMA3 = 11,
    Keypad = 12,
    Slot2 = 13,
    IPCSync = 16,
    IPCEmpty = 17,
    IPCNonEmpty = 18,
    Slot1TransferComplete = 19,
    Slot1IREQMC = 20,

    HingeOpen = 22,
    SPI = 23,
    Wifi = 24,

    NDMA0 = 28,
    NDMA1 = 29,
    NDMA2 = 30,
    NDMA3 = 31,

    GPIO180 = 0 + AUX_INTERRUPT,
    GPIO181 = 1 + AUX_INTERRUPT,
    GPIO182 = 2 + AUX_INTERRUPT,

    HeadphoneConnect = 5 + AUX_INTERRUPT,
    Powerbutton = 6 + AUX_INTERRUPT,
    SoundEnableOutput = 7 + AUX_INTERRUPT,
    SDMMC = 8 + AUX_INTERRUPT,
    SDMMCData1 = 9 + AUX_INTERRUPT,
    SDIO = 10 + AUX_INTERRUPT,
    SDIOData1 = 11 + AUX_INTERRUPT,
    AES = 12 + AUX_INTERRUPT,
    I2C = 13 + AUX_INTERRUPT,
    MicrophoneExt = 14 + AUX_INTERRUPT,
}
pub unsafe fn set_interrupt_function(interrupt: ARM7Interrupt, function: unsafe fn()) {
    crate::critical_function(|| {
        let interrupt = interrupt as u8;
        let index = interrupt & INTERRUPT_INDEX_MASK;
        if interrupt > INTERRUPT_INDEX_MASK {
            INTERRUPT_TABLE_AUX[index as usize] = function as *mut _;
        } else {
            INTERRUPT_TABLE[index as usize] = function as *mut _;
        }
    });
}
pub unsafe fn enable_interrupt(interrupt: ARM7Interrupt) {
    let interrupt = interrupt as u8;
    let index = interrupt & INTERRUPT_INDEX_MASK;
    let fun = if interrupt > INTERRUPT_INDEX_MASK {
        crate::critical_function(|| {
            super::INTERUPT_HARDWARE
                .enable2
                .modify(|i| i | (1 << index))
        });
    } else {
        crate::critical_function(|| super::INTERUPT_HARDWARE.enable.modify(|i| i | (1 << index)));
    };
}
pub unsafe fn disable_all_interrupts() {
    (0x400_0208 as *mut u32).write_volatile(0);
}
pub unsafe fn disable_interrupt(interrupt: ARM7Interrupt) {
    let interrupt = interrupt as u8;
    let index = interrupt & INTERRUPT_INDEX_MASK;
    if interrupt > INTERRUPT_INDEX_MASK {
        super::INTERUPT_HARDWARE
            .enable2
            .modify(|i| i & !(1 << index));
    } else {
        super::INTERUPT_HARDWARE
            .enable
            .modify(|i| i & !(1 << index));
    }
}

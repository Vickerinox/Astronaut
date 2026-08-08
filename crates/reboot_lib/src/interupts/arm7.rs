// SPDX-FileCopyrightText: 2026 Viktor Karlsson <viktor@koda.re>
// SPDX-License-Identifier: MIT

/// A interrupt handler appropriate for the ds, courtesy of libnds
#[cfg(feature = "arm7i")]
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
        "beq 3f", // if there are "no interrupts" to be serviced, it's gotta be the aux ones!

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

            // load irq table and jump to function pointer
            "ldr r3, ={irq_table}",
            "add r3, r0, lsl #2",

            "b 4f",

        //check AUX irq's, in case it wasn't a standard IRQ
        "3:",
            "ldr r1, [r12, {i_ae}]",
            "ldr r2, [r12, {i_af}]",
            "ands r1, r1, r2", //the interrupt bits to be serviced! (i.e IE & IF)
            "moveq pc, lr", // EARLY RETURN: There are truly no IRQ's to service!

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

            // load irq table and jump to function pointer
            "ldr r3, ={irq_table_aux}",
            "add r3, r0, lsl #2",
        //Dereference the interrupt function pointer
        "4:",
        "ldr r3, [r3]",
        "cmp r3, #0",
        "beq 2f", //EARLY RETURN: no interrupt handler installed, do nothing instead.
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

        //Back to business.
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
unsafe fn interrupt_handler_arm7() {
    panic!()
}
/// Initialize the interrupt hardware such as that no interrupts are enabled and all interrupts are cleared.
#[cfg(feature = "arm7")]
pub unsafe fn init_interrupts() {
    use crate::INTERRUPT_HARDWARE;
    INTERRUPT_HARDWARE.master.write(0);
    INTERRUPT_HARDWARE.enable.write(0);
    INTERRUPT_HARDWARE.request.write(!0);
    INTERRUPT_HARDWARE.enable2.write(0);
    INTERRUPT_HARDWARE.request2.write(!0);
    (0x0380_FFFC as *mut unsafe fn()).write(interrupt_handler_arm7);
    INTERRUPT_HARDWARE.master.write(1);
}
use crate::interupts::INTERRUPT_INDEX_MASK;
use crate::interupts::INTERRUPT_TABLE;
use crate::Interrupt;

#[cfg(feature = "arm7i")]
use crate::interupts::INTERRUPT_TABLE_AUX;

/// Bind a function to a given [`Interrupt`]
pub unsafe fn set_interrupt_function(interrupt: Interrupt, function: unsafe fn()) {
    crate::critical_function(|| {
        let interrupt = interrupt as u8;
        #[cfg(feature = "arm7i")]
        {
            let index = interrupt & INTERRUPT_INDEX_MASK;
            if interrupt > INTERRUPT_INDEX_MASK {
                INTERRUPT_TABLE_AUX[index as usize] = function as *mut _;
            } else {
                INTERRUPT_TABLE[index as usize] = function as *mut _;
            }
        }
        #[cfg(not(feature = "arm7i"))]
        {
            INTERRUPT_TABLE[interrupt as usize] = function as *mut _;
        }
    });
}
/// Enable a given [`Interrupt`].
/// 
/// This will cause the function which is binded with [`set_interrupt_function`] to be called any time an 
/// [`Interrupt`] of this type is fired. If the interrupt has no bound function, this will do nothing*.
/// 
/// \* functions such as [`swi_halt`](crate::swi::swi_halt) are awoken by any interrupt firing, even when theres no binding function.
pub unsafe fn enable_interrupt(interrupt: Interrupt) {
    let interrupt = interrupt as u8;
    #[cfg(feature = "arm7i")]
    {
        let index = interrupt & INTERRUPT_INDEX_MASK;
        let fun = if interrupt > INTERRUPT_INDEX_MASK {
            crate::critical_function(|| {
                super::INTERRUPT_HARDWARE
                    .enable2
                    .modify(|i| i | (1 << index))
            });
        } else {
            crate::critical_function(|| {
                super::INTERRUPT_HARDWARE.enable.modify(|i| i | (1 << index))
            });
        };
    }
    #[cfg(not(feature = "arm7i"))]
    {
        crate::critical_function(|| {
            super::INTERRUPT_HARDWARE
                .enable
                .modify(|i| i | (1 << interrupt))
        });
    }
}
/// Disables all interrupts via the master interrupt enable register.
pub unsafe fn disable_all_interrupts() {
    (0x400_0208 as *mut u32).write_volatile(0);
}
/// Disable a given [`Interrupt`] from firing.
pub unsafe fn disable_interrupt(interrupt: Interrupt) {
    let interrupt = interrupt as u8;
    let index = interrupt & INTERRUPT_INDEX_MASK;
    if interrupt > INTERRUPT_INDEX_MASK {
        super::INTERRUPT_HARDWARE
            .enable2
            .modify(|i| i & !(1 << index));
    } else {
        super::INTERRUPT_HARDWARE
            .enable
            .modify(|i| i & !(1 << index));
    }
}

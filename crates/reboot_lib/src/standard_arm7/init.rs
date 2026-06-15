use crate::{
    spi::{Control, PowerRegiser},
    swi_delay,
};

pub unsafe fn init_power_regs() {
    (0x400_0304 as *mut u32).write_volatile(1);
}
pub unsafe fn init_i2c() {
    crate::i2c::init();
}
pub unsafe fn init_ntr_sound() {
    crate::sound::SOUND_HARDWARE.init();
    swi_delay(0x20BA * 16);
}
pub unsafe fn init_powerman() {
    crate::spi::write_powerman(PowerRegiser::Control(Control::ENABLE_SOUND_AMP));
}
pub unsafe fn init_powerman2() {
    crate::spi::write_powerman(PowerRegiser::Control(
        Control::ENABLE_BACKLIGHTS | Control::ENABLE_SOUND_AMP,
    ));
}
pub unsafe fn init_nwram() {
    (0x4004060 as *mut u32).write_volatile(0);
}

use core::ops::Index;

use volatile_register::RW;

use crate::MemoryWrapper;

pub const TIMERS: MemoryWrapper<Timers> = MemoryWrapper(0x4000100 as *mut Timers);

#[repr(C)]
pub struct Timers([RW<Timer>; 4]);

#[derive(Clone, Copy)]
#[repr(C, align(4))]
pub struct Timer(u32);
impl Timer {
    pub const fn new(reload: u16, control: TimerControl) -> Self {
        Self(reload as u32 | ((control.bits() as u32) << 16))
    }
    pub const RESET: Self = Self::new(0, TimerControl::empty());
}
impl Index<usize> for Timers {
    type Output = RW<Timer>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}
impl Timers {
    pub unsafe fn clear(&self) {
        for timer in &self.0 {
            timer.write(Timer::RESET);
        }
    }
}

bitflags::bitflags! {
    #[derive(Clone, Copy)]
    pub struct TimerControl: u16 {
        const ENABLE_IRQ = (1<<6);
        const START = (1<<7);
        const PRESCALE_1 = 0;
        const PRESCALE_64 = 1;
        const PRESCALE_256 = 2;
        const PRESCALE_1024 = 3;
        //note! prescale does nothing when countup is useds
        const USE_COUNTUP = (1<<2);

    }
}

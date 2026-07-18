// SPDX-FileCopyrightText: 2026 Viktor Karlsson <viktor@koda.re>
// SPDX-License-Identifier: GPL-3.0-or-later

mod module;
mod wav;
pub use module::{send_mod_file, stop_mod_file};
use reboot_lib::{music_modules::mods::MODAsyncLoader, timers::TimerControl, Interrupt};
pub use wav::StreamingWav;

use crate::{AppArea, APP_AREA_START};
pub enum MusicPlaying {
    None,
    Mod(MODAsyncLoader),
    Wav(StreamingWav),
}

unsafe fn uptick_wav() {
    (*(APP_AREA_START as *mut AppArea))
        .wav_counter
        .modify(|i| i + 1);
}

pub unsafe fn init() {
    reboot_lib::timers::TIMERS[0].write(reboot_lib::timers::Timer::new(0, TimerControl::empty()));
    reboot_lib::set_interrupt_function(Interrupt::Timer0, uptick_wav);
    reboot_lib::enable_interrupt(Interrupt::Timer0);
}

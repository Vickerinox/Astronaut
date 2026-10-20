// SPDX-FileCopyrightText: 2026 Viktor Karlsson <viktor@koda.re>
// SPDX-License-Identifier: GPL-3.0-or-later

use reboot_lib::{volatile_register, VIDEO_HARDWARE};

use crate::{AppArea, APP_AREA_START};

pub struct Fader {
    current_top: reboot_lib::volatile_register::RW<i8>,
    target_top: reboot_lib::volatile_register::RW<i8>,
    current_bottom: reboot_lib::volatile_register::RW<i8>,
    target_bottom: reboot_lib::volatile_register::RW<i8>,
}

impl Fader {
    pub fn is_up(&self) -> bool {
        self.current_top.read() > 15 && self.current_bottom.read() > 15
    }
    pub fn set_target(&mut self, target: i8) {
        self.set_target_bottom(target);
        self.set_target_top(target);
    }
    pub fn set_state(&mut self, target: i8) {
        self.set_target(target);
        unsafe {
            self.current_bottom.write(target);
            self.current_top.write(target);
        }
    }
    pub fn set_target_bottom(&mut self, target: i8) {
        unsafe {
            self.target_bottom.write(target);
        }
    }
    pub fn set_target_top(&mut self, target: i8) {
        unsafe {
            self.target_top.write(target);
        }
    }
    pub fn synced_top(&self) -> bool {
        self.current_top.read() == self.target_top.read()
    }
    pub fn synced_bottom(&self) -> bool {
        self.current_bottom.read() == self.target_bottom.read()
    }
    pub fn synced(&self) -> bool {
        self.synced_bottom() && self.synced_top()
    }
}

#[link_section = ".text_itcm"]
pub unsafe fn irq_screen_fade() {
    let area = &mut (*(APP_AREA_START as *mut AppArea)).app_data.assume_init_mut().global_data.fader;
    unsafe fn fade(
        target: &mut volatile_register::RW<i8>,
        current: &mut volatile_register::RW<i8>,
        reg: &mut volatile_register::RW<u16>,
    ) {
        let read = current.read();
        let target = target.read();
        let new = match read.cmp(&target) {
            core::cmp::Ordering::Less => (read + 3).min(target),
            core::cmp::Ordering::Equal => return,
            core::cmp::Ordering::Greater => (read - 2).max(target),
        };
        current.write(new);
        reg.write(new as u16 | (1 << 14));
    }
    #[allow(const_item_mutation)]
    {
        fade(
            &mut area.target_top,
            &mut area.current_top,
            &mut VIDEO_HARDWARE.disp_b_master_bright,
        );
        fade(
            &mut area.target_bottom,
            &mut area.current_bottom,
            &mut VIDEO_HARDWARE.master_brightness,
        );
    }
}

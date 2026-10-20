// SPDX-FileCopyrightText: 2026 Viktor Karlsson <viktor@koda.re>
// SPDX-License-Identifier: GPL-3.0-or-later

pub mod bmp;
pub mod font;

mod init;

pub use init::init_graphics;
use reboot_lib::{ENGINE_A_PALETTES, ENGINE_B_PALETTES};

use crate::{graphics::bmp::AsyncBMPLoader, gui::GlobalData};

pub unsafe fn set_background(color: u16) {
    ENGINE_A_PALETTES.bg_palettes[0].write(color);
    ENGINE_B_PALETTES.bg_palettes[0].write(color);
}
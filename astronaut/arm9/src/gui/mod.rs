// SPDX-FileCopyrightText: 2026 Viktor Karlsson <viktor@koda.re>
// SPDX-License-Identifier: GPL-3.0-or-later

pub use micro_imgui_ds::gui::DSMicroGuiBackend;

mod backend;
pub use frontend::{AppData, GlobalData};
use micro_imgui_ds::{micro_imgui::InputEvent, Input};
use reboot_lib::Buttons;
mod browser;
mod error;
mod frontend;
mod main_menu;
mod special_thanks;
pub use main_menu::MainMenu;
mod settings;
use crate::Fader;
pub use frontend::pop_dir_entry;
use micro_imgui_ds::micro_imgui::Backend;

#[no_mangle]
#[link_section = ".text_aux"]
pub unsafe fn load_gui(app_data: &mut AppData, fader: &mut Fader, buttons: Buttons) {
    let (assets, style) = app_data
        .global_data
        .theme
        .load(&mut app_data.global_data.config.theme_path);
    let video_context = app_data.global_data.load_theme(assets);
    let backend = micro_imgui_ds::DSMicroGuiBackend::new(video_context, buttons);

    fader.target.write(0);

    micro_imgui_ds::micro_imgui::run(
        backend,
        style,
        app_data,
        |mut f, app_data| {
            app_data.update(&mut f);
        },
        |app_data| {
            app_data.do_background_tasks();
        },
    );
}
pub fn focus_default(
    ui: &mut micro_imgui_ds::micro_imgui::Ui<'_, '_, micro_imgui_ds::DSMicroGuiBackend>,
) {
    if ui.input_pressed(Input::FOCUS_NEXT)
        || (!ui.has_focus_anywhere() && !ui.input_pressed(Input::FOCUSED_PRESS))
    {
        ui.focus_next();
    } else if ui.input_pressed(Input::FOCUS_PREVIOUS) {
        ui.focus_prev();
    }
}
pub fn show_wallpaper(bmp: crate::bmp::DecodedBMP, destination: *mut u16) {
    if bmp.height() != 192 {
        return;
    }
    if bmp.width() != 256 {
        return;
    }
    let paletter = bmp.palette_table();
    let a = |chunk: &[u8]| {
        let red = paletter[((chunk[0] as usize) << 2) + 0] >> 3;
        let green = paletter[((chunk[0] as usize) << 2) + 1] >> 3;
        let blue = paletter[((chunk[0] as usize) << 2) + 2] >> 3;
        0x8000 | ((red as u16) << 10) | ((green as u16) << 5) | (blue as u16)
    };
    let b = |chunk: &[u8]| {
        let red = chunk[0] >> 3;
        let green = chunk[1] >> 3;
        let blue = chunk[2] >> 3;
        0x8000 | ((red as u16) << 10) | ((green as u16) << 5) | (blue as u16)
    };
    let pixel_iter: core::iter::Map<core::slice::ChunksExact<'_, u8>, &dyn Fn(&[u8]) -> u16> =
        match (bmp.dib.bits_per_pixel, bmp.dib.compression) {
            (16, 3) => {
                if bmp.palette_table()
                    != &[
                        00, 0x7C, 0x00, 0x00, 0xE0, 0x03, 0x00, 0x00, 0x1F, 0x00, 0x00, 0x00,
                    ]
                {
                    return;
                }
                bmp.bitmap.chunks_exact(2).map(&|chunk| {
                    let pixel = u16::from_le_bytes([chunk[0], chunk[1]]);
                    let red = pixel & 0x1F;
                    let green = (pixel & (0x1F << 5)) >> 5;
                    let blue = (pixel & (0x1F << 10)) >> 10;
                    0x8000 | (red << 10) | (green << 5) | (blue)
                })
            }
            (8, 0) => bmp.bitmap.chunks_exact(1).map(&a),
            (32, 3) => {
                if bmp.palette_table()
                    != &[
                        00, 0x00, 0xFF, 0x00, 0x00, 0xFF, 0x00, 0x00, 0xFF, 0x00, 0x00, 0x00,
                    ]
                {
                    return;
                }
                bmp.bitmap.chunks_exact(4).map(&b)
            }
            (24, 0) => bmp.bitmap.chunks_exact(3).map(&b),
            _ => return,
        };
    unsafe {
        for (i, pixel) in pixel_iter.enumerate() {
            destination.add(i).write(pixel);
        }
    }
}

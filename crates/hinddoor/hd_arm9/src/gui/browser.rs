use core::ops::{Add, Sub};

use alloc::{boxed::Box, format, string::String, vec::Vec};
use fatfs_embedded::fatfs::FileOptions;
use micro_imgui_ds::{Input, SCREEN_RECT, micro_imgui::{self, Color, Sizing, Vec2, widgets::button::Button}};
use reboot_lib::{Buttons, music_modules::mods::MODAsyncLoader};

use crate::{BACKGROUND_COLOR, COLOR_BOOTABLE, COLOR_MUSIC, get_extension, gui::{AppData, CurrentFrontend, MusicPlaying, frontend::{AppBooter, StreamingWav, UiPage, pop_dir_entry}, main_menu::MainMenu}, populate_fs_vec, stop_mod_file};
impl AppData {
    pub fn open_sd() -> Option<Browser> {
        let mut file_path = String::from("sdmc:/");
        fatfs_embedded::opendir(&mut file_path).ok().map(|mut i| {
            let immediate_files = populate_fs_vec(&mut i);
            Browser {
                immediate_files,
                file_path,
                offset: 0,
                drag_start: 0,
                hold_timer: 0,
            }
        })
    }
    pub fn open_nand() -> Option<Browser> {
        let mut file_path = String::from("nand:/");
        fatfs_embedded::opendir(&mut file_path).ok().map(|mut i| {
            let immediate_files = populate_fs_vec(&mut i);
            Browser {
                immediate_files,
                file_path,
                offset: 0,
                drag_start: 0,
                hold_timer: 0,
            }
        })
    }
}
pub struct Browser {
    immediate_files: Vec<(String, String, bool, Color)>,
    file_path: String,
    offset: i32,
    drag_start: i16,
    hold_timer: i16,
}
impl UiPage for Browser {
    fn ui(&mut self, ui: &mut micro_imgui_ds::micro_imgui::Ui<'_, '_, micro_imgui_ds::DSMicroGuiBackend>, data: &mut super::GlobalData) -> Option<Box<dyn UiPage>> {
        let Self {
                    immediate_files,
                    file_path: current_path,
                    offset,
                    drag_start,
                    hold_timer,
                } = self;

                    const ITEM_SPACING: i32 = 14;

                    let max_scroll = ((immediate_files.len()) as i32 * ITEM_SPACING)
                                    - (ITEM_SPACING * 11);
                    if let Some(drag) = ui.drag() {
                        let new_drag = drag.y - *drag_start;
                        *drag_start += new_drag;
                        *offset -= new_drag as i32;
                        *offset = (*offset)
                            .min(max_scroll)
                            .max(0);
                    } else {
                        *drag_start = 0;
                    }

                    let mut focus_on = None;
                    if ui.input_pressed(Input(Buttons::DIRECTION_RIGHT)) {
                        focus_on = Some(10);
                    }
                    if ui.input_pressed(Input(Buttons::DIRECTION_LEFT)) {
                        focus_on = Some(0);
                    }

                    let shown_items = immediate_files
                        .get(((*offset / ITEM_SPACING) as usize)..)
                        .unwrap_or(&[]);

                    let in_step = *offset % ITEM_SPACING;

                    let mut control_dir = 0;
                    if ui.input_pressed(Input(Buttons::DIRECTION_UP)) {
                        control_dir = 1;
                    }

                    if ui.input_pressed(Input(Buttons::DIRECTION_DOWN)) {
                        control_dir = -1;
                    }
                    if ui.input_down(Input(Buttons::DIRECTION_UP)) && ui.has_focus_anywhere() {
                        *hold_timer += 1;
                        ui.request_repaint();
                    } else if ui.input_down(Input(Buttons::DIRECTION_DOWN))
                        && ui.has_focus_anywhere()
                    {
                        *hold_timer -= 1;
                        ui.request_repaint();
                    } else {
                        *hold_timer = 0;
                    }
                    if hold_timer.abs() > 30 && (*hold_timer & 1 == 0) {
                        if hold_timer.is_negative() {
                            ui.focus_next();
                            control_dir = -1;
                        } else {
                            ui.focus_prev();
                            control_dir = 1
                        }
                    }

                    let mut new_state: Option<Box<dyn UiPage>> = None;
                    let mut new_folder = None;
                    ui.label(current_path);
                    let rect = micro_imgui::Rect::from_two_pos(
                        ui.clip_rect().top_left(),
                        ui.clip_rect().top_right() + Vec2::new(0, ITEM_SPACING as _),
                    );
                    let rect2 = micro_imgui::Rect::from_two_pos(
                        SCREEN_RECT.bottom_left() - Vec2::new(0, 5 as _),
                        SCREEN_RECT.bottom_right(),
                    );

                    let color = BACKGROUND_COLOR;

                    ui.add_space((ITEM_SPACING - in_step) as i16);
                    let items = if in_step == 0 { 11 } else { 12 };
                    let mut focus = None;
                    for (i, item) in shown_items.iter().take(items).enumerate() {
                        let response = ui.add(Button::new(
                            &item.0,
                            Sizing::Padded(Vec2::new(248, 8)),
                            item.3,
                        ));
                        if response.focused() {
                            focus = Some(i);
                        }
                        if Some(i) == focus_on {
                            ui.set_focus(&response);
                            ui.request_repaint();
                        }
                        if response.clicked() {
                            if item.2 {
                                current_path.push_str(&item.1);
                                current_path.push('/');
                                if let Ok(f) = fatfs_embedded::opendir(current_path) {
                                    new_folder = Some(f);
                                }
                            } else {
                                if item.3 == COLOR_BOOTABLE {
                                    current_path.push_str(&item.1);
                                    match fatfs_embedded::open(current_path, FileOptions::Read) {
                                        Ok(file) => {
                                            let bajs = current_path.clone();
                                            new_state = Some(Box::new(AppBooter { path: bajs}));
                                        }
                                        Err(_) => (),
                                    }
                                } else if item.3 == COLOR_MUSIC {
                                    match get_extension(item.1.as_bytes()) {
                                        Some(b".mod") => {                                        
                                            current_path.push_str(&item.1);
                                            match fatfs_embedded::open(current_path, FileOptions::Read) {
                                                Ok(module) => {
                                                    data.loading_mod_file = MusicPlaying::None;
                                                    data.loading_mod_file = MusicPlaying::Mod(MODAsyncLoader::new(module));
                                                }
                                                Err(_abort) => (),
                                            }
                                            pop_dir_entry(current_path);
                                        }
                                        Some(b".wav") => {
                                            let _ = stop_mod_file();
                                            current_path.push_str(&item.1);
                                            match fatfs_embedded::open(current_path, FileOptions::Read) {
                                                Ok(module) => {
                                                    if let Some(mut wav) = StreamingWav::new(module) {
                                                        data.loading_mod_file = MusicPlaying::None;
                                                        unsafe { wav.play() };
                                                        data.loading_mod_file = MusicPlaying::Wav(wav);
                                                        ui.request_repaint();

                                                    }
                                                    
                                                }
                                                Err(_abort) => (),
                                            }
                                            pop_dir_entry(current_path);
                                            

                                        }
                                        _ => (),
                                    }
                                }
                            }
                        }
                    }
                    ui.paint_shape(micro_imgui::Shape::Rectangle {
                        area: rect,
                        fill: Color(color),
                        rounding: 0,
                        outline_color: Color(color),
                        outline_size: 0,
                    });
                    ui.paint_shape(micro_imgui::Shape::Rectangle {
                        area: rect2,
                        fill: Color(color),
                        rounding: 0,
                        outline_color: Color(color),
                        outline_size: 0,
                    });

                    if focus == focus_on {
                        if focus_on == Some(0) {
                            *offset = (*offset).sub(ITEM_SPACING * 10).max(0);
                        }
                        if focus_on == Some(10) {
                            *offset = (*offset)
                                .add(ITEM_SPACING * 10)
                                .min(max_scroll)
                                .max(0);
                        }
                    }
                    if control_dir == 1 {
                        if focus == Some(0) && *offset > 0 {
                            *offset = offset.wrapping_sub(ITEM_SPACING).max(0);
                            ui.cancel_refocus();
                        }

                        *offset -= in_step;
                    } else if control_dir == -1 {
                        if focus == Some(10) && shown_items.len() >= 12 {
                            *offset = offset.saturating_add(ITEM_SPACING);
                            ui.cancel_refocus();
                        }

                        *offset -= in_step;
                    }
                    if ui.input_pressed(Input(Buttons::BUTTON_B)) && new_folder.is_none() {
                        if ["nand:/", "sdmc:/"].contains(&current_path.as_str()) {
                            new_state = Some(Box::new(MainMenu));
                        } else {
                            pop_dir_entry(current_path);
                            match fatfs_embedded::opendir(current_path) {
                                Ok(f) => {
                                    new_folder = Some(f);
                                }
                                Err(_err) => {
                                    new_state = Some(Box::new(super::error::Error { error_string: format!("Filesystem error!") }));
                                }
                            }
                        }
                    }
                    if let Some(mut new_folder) = new_folder {
                        *immediate_files = populate_fs_vec(&mut new_folder);
                        *offset = 0;
                    }
                    new_state
                
    }
}
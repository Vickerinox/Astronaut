// SPDX-FileCopyrightText: 2026 Viktor Karlsson <viktor@koda.re>
// SPDX-License-Identifier: GPL-3.0-or-later

use core::ops::{Add, Sub};

use alloc::{boxed::Box, format, string::String, vec::Vec};
use fatfs_embedded::fatfs::FileOptions;
use micro_imgui_ds::{
    micro_imgui::{self, widgets::button::Button, Backend, Sizing, Vec2},
    Input, SCREEN_RECT,
};
use reboot_lib::fatfs_embedded;
use reboot_lib::{music_modules::mods::MODAsyncLoader, Buttons};

use crate::{
    filetype,
    gui::{
        frontend::{pop_dir_entry, AppBooter, ClonableUiPage, UiPage},
        main_menu::MainMenu,
        GlobalData,
    },
    music::{stop_mod_file, MusicPlaying, StreamingWav},
    truncate_name, FileEntry, FileType,
};


pub fn populate_fs_vec(folder: &mut fatfs_embedded::fatfs::Directory) -> Vec<FileEntry> {
    let mut vec: Vec<_> = alloc::vec::Vec::new();

    loop {
        if let Ok(file) = fatfs_embedded::readdir(folder) {
            let Ok(name) = unsafe { core::ffi::CStr::from_ptr(file.fname.as_ptr()) }.to_str()
            else {
                continue;
            };
            let name = alloc::string::String::from(name);
            if name.is_empty() {
                break;
            }
            let is_dir = file.fattrib & fatfs_embedded::fatfs::FileAttributes::Directory.bits() > 0;
            let color = if is_dir {
                FileType::Dir
            } else {
                let s_name = unsafe { core::ffi::CStr::from_ptr(file.altname.as_ptr()).to_bytes() };
                let s_name = if s_name.is_empty() {
                    name.as_bytes()
                } else {
                    s_name
                };
                filetype(s_name)
            };
            let dname = truncate_name(&name, 35);
            vec.push(FileEntry {
                display_name: dname,
                file_name: name,
                kind: color,
            })
        } else {
            panic!("SD card seems to have been ejected!");
        }
    }

    for i in 1..vec.len() {
        let Some(temp) = vec.get(i) else { break };
        let temp = temp.clone();
        let mut j = i;
        loop {
            let Some(under) = vec.get(j - 1) else { break };
            if under.kind > temp.kind {
                let under = under.clone();
                let Some(over) = vec.get_mut(j) else { break };
                *over = under;
                j -= 1;
            } else {
                break;
            }
        }
        vec[j] = temp;
    }

    vec
}

impl Browser {
    pub fn open_sd() -> Option<Browser> {
        Self::open_browser(Mode::Browsing, Box::new(MainMenu), String::from("sdmc:/"))
    }
    pub fn open_nand() -> Option<Browser> {
        Self::open_browser(Mode::Browsing, Box::new(MainMenu), String::from("nand:/"))
    }
    pub fn open_browser(
        mode: Mode,
        exit: Box<dyn ClonableUiPage>,
        mut open_path: String,
    ) -> Option<Browser> {
        fatfs_embedded::opendir(&mut open_path).ok().map(|mut i| {
            let immediate_files = populate_fs_vec(&mut i);
            Browser {
                immediate_files,
                current_path: open_path,
                offset: 0,
                drag_start: 0,
                hold_timer: 0,
                mode,
                exit,
            }
        })
    }
}
pub struct Browser {
    immediate_files: Vec<FileEntry>,
    current_path: String,
    offset: i32,
    drag_start: i16,
    hold_timer: i16,
    mode: Mode,
    exit: Box<dyn ClonableUiPage>,
}
#[derive(Clone)]
pub enum Mode {
    Browsing,
    Searching(BrowserSearch),
}
#[derive(Clone)]
pub struct BrowserSearch {
    pub filter: &'static [FileType],
    pub goal: &'static dyn Fn(&mut GlobalData, String) -> Option<Box<dyn UiPage>>,
}

impl Browser {
    pub fn search_file(
        format: &'static [FileType],
        start: String,
        exit: Box<dyn ClonableUiPage>,
        transform: &'static dyn Fn(&mut GlobalData, String) -> Option<Box<dyn UiPage>>,
    ) -> Option<Browser> {
        Browser::open_browser(
            Mode::Searching(BrowserSearch {
                filter: format,
                goal: transform,
            }),
            exit,
            start,
        )
    }
    fn open_new(&self, file_name: &str) -> Option<Box<dyn UiPage>> {
        let mut new_folder = self.current_path.clone() + file_name + "/";
        if let Ok(mut f) = fatfs_embedded::opendir(&mut new_folder) {
            Some(Box::new(Self {
                immediate_files: populate_fs_vec(&mut f),
                current_path: new_folder,
                offset: 0,
                drag_start: 0,
                hold_timer: 0,
                exit: self.exit.clone_ui(),
                mode: self.mode.clone(),
            }))
        } else {
            None
        }
    }
    fn standard_goal(&self, file: &FileEntry, data: &mut GlobalData) -> Option<Box<dyn UiPage>> {
        let FileEntry {
            file_name, kind, ..
        } = file;
        match *kind {
            FileType::Dir => self.open_new(file_name),
            FileType::Rom => {
                let path = self.current_path.clone() + file_name;
                Some(Box::new(AppBooter { path }))
            }
            FileType::Mod => {
                let mut path = self.current_path.clone() + file_name;
                match fatfs_embedded::open(&mut path, FileOptions::Read) {
                    Ok(module) => {
                        data.loading_mod_file = MusicPlaying::None;
                        data.loading_mod_file = MusicPlaying::Mod(MODAsyncLoader::new(module));
                    }
                    Err(_abort) => (),
                }
                None
            }
            FileType::Wav => {
                let _ = stop_mod_file();
                let mut path = self.current_path.clone() + file_name;
                match fatfs_embedded::open(&mut path, FileOptions::Read) {
                    Ok(module) => {
                        if let Some(mut wav) = StreamingWav::new(module) {
                            data.loading_mod_file = MusicPlaying::None;
                            unsafe { wav.play() };
                            data.loading_mod_file = MusicPlaying::Wav(wav);
                        }
                    }
                    Err(_abort) => (),
                }
                None
            }
            _ => None,
        }
    }
}
impl UiPage for Browser {
    fn ui(
        &mut self,
        ui: &mut micro_imgui_ds::micro_imgui::Ui<'_, '_, micro_imgui_ds::DSMicroGuiBackend>,
        data: &mut super::GlobalData,
    ) -> Option<Box<dyn UiPage>> {
        const ITEM_SPACING: i32 = 14;

        let max_scroll = ((self.immediate_files.len()) as i32 * ITEM_SPACING) - (ITEM_SPACING * 11);
        if let Some(drag) = ui.drag() {
            let new_drag = drag.y - self.drag_start;
            self.drag_start += new_drag;
            self.offset -= new_drag as i32;
            self.offset = (self.offset).min(max_scroll).max(0);
        } else {
            self.drag_start = 0;
        }

        let mut focus_on = None;
        if ui.input_pressed(Input(Buttons::DIRECTION_RIGHT)) {
            focus_on = Some(10);
        }
        if ui.input_pressed(Input(Buttons::DIRECTION_LEFT)) || !ui.has_focus_anywhere() {
            focus_on = Some(0);
        }

        let shown_items = self
            .immediate_files
            .get(((self.offset / ITEM_SPACING) as usize)..)
            .unwrap_or(&[]);

        let in_step = self.offset % ITEM_SPACING;

        if ui.input_down(Input(Buttons::DIRECTION_UP)) {
            self.hold_timer += 1;
            ui.request_repaint();
        } else if ui.input_down(Input(Buttons::DIRECTION_DOWN)) {
            self.hold_timer -= 1;
            ui.request_repaint();
        } else {
            self.hold_timer = 0;
        }

        let mut new_state: Option<Box<dyn UiPage>> = None;
        let mut new_folder = None;
        ui.label(&self.current_path);
        let rect = micro_imgui::Rect::from_two_pos(
            ui.clip_rect().top_left(),
            ui.clip_rect().top_right() + Vec2::new(0, ITEM_SPACING as _),
        );
        let rect2 = micro_imgui::Rect::from_two_pos(
            SCREEN_RECT.bottom_left() - Vec2::new(0, 5 as _),
            SCREEN_RECT.bottom_right(),
        );

        let color = ui.style().background_color;

        ui.add_space((ITEM_SPACING - in_step) as i16);
        let items = if in_step == 0 { 11 } else { 12 };
        let mut focus = None;
        for (i, item) in shown_items.iter().take(items).enumerate() {
            let color = match item.kind {
                FileType::None => ui.style().text_color,
                FileType::Rom => data.theme.bootable_color,
                FileType::Mod => data.theme.asset_color,
                FileType::Wav => data.theme.asset_color,
                FileType::Bmp => data.theme.asset_color,
                FileType::Ini => data.theme.asset_color,
                FileType::Dir => data.theme.folder_color,
            };
            let response = ui.add(Button::new(
                &item.display_name,
                Sizing::Padded(Vec2::new(248, 8)),
                color,
            ));
            if response.focused() {
                focus = Some(i);
            }
            if Some(i) == focus_on {
                ui.set_focus(&response);
                ui.request_repaint();
            }
            if response.clicked() {
                match &self.mode {
                    Mode::Browsing => new_state = self.standard_goal(&item, data),
                    Mode::Searching(BrowserSearch { filter, goal }) => {
                        new_state = if filter.contains(&item.kind) {
                            goal(data, self.current_path.clone() + &item.file_name)
                        } else if item.kind == FileType::Dir {
                            self.open_new(&item.file_name.clone())
                        } else {
                            None
                        }
                    }
                }
            }
        }
        ui.paint_shape(micro_imgui::Shape::Rectangle {
            area: rect,
            fill: color,
            rounding: 0,
            outline_color: color,
            outline_size: 0,
        });
        ui.paint_shape(micro_imgui::Shape::Rectangle {
            area: rect2,
            fill: color,
            rounding: 0,
            outline_color: color,
            outline_size: 0,
        });

        if focus == focus_on {
            if focus_on == Some(0) {
                self.offset = (self.offset).sub(ITEM_SPACING * 10).max(0);
            }
            if focus_on == Some(10) {
                self.offset = (self.offset).add(ITEM_SPACING * 10).min(max_scroll).max(0);
            }
            self.offset -= in_step;
        }
        if (self.hold_timer.abs() > 30 && (self.hold_timer & 1 == 0)) || self.hold_timer.abs() == 1
        {
            if self.hold_timer.is_negative() {
                if focus == Some(10) {
                    if shown_items.len() >= 12 {
                        self.offset = self.offset.saturating_add(ITEM_SPACING);
                    }
                } else {
                    ui.focus_next();
                }
                self.offset -= in_step;
            } else {
                if focus == Some(0) {
                    if self.offset > 0 {
                        self.offset = self.offset.wrapping_sub(ITEM_SPACING).max(0);
                    }
                } else {
                    ui.focus_prev();
                }
                self.offset -= in_step;
            }
        }
        if ui.input_pressed(Input(Buttons::BUTTON_B)) && new_folder.is_none() {
            if ["nand:/", "sdmc:/"].contains(&self.current_path.as_str()) {
                new_state = Some(self.exit.clone_ui());
            } else {
                pop_dir_entry(&mut self.current_path);
                match fatfs_embedded::opendir(&mut self.current_path) {
                    Ok(f) => {
                        new_folder = Some(f);
                    }
                    Err(_err) => {
                        new_state = Some(Box::new(super::error::Error::new(format!(
                            "Filesystem error!"
                        ))));
                    }
                }
            }
        }
        if let Some(mut new_folder) = new_folder {
            self.immediate_files = populate_fs_vec(&mut new_folder);
            self.offset = 0;
        }
        new_state
    }
}

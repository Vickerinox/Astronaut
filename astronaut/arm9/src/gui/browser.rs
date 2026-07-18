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

    // scan the dir for all the files
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

    // sort all the entries
    for i in 1..vec.len() {
        let Some(temp) = vec.get(i) else { break };
        let temp = temp.clone();
        let mut j = i;
        loop {
            let Some(under) = vec.get(j - 1) else { break };
            if under.partial_cmp(&temp) == Some(core::cmp::Ordering::Greater) {
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
    // Opens a version of the browser which lets the user browse the SD
    pub fn open_sd() -> Option<Browser> {
        Self::open_browser(
            BrowserMode::Browsing,
            Box::new(MainMenu),
            String::from("sdmc:/"),
        )
    }

    // Opens a version of the browser which lets the user browse the NAND
    pub fn open_nand() -> Option<Browser> {
        Self::open_browser(
            BrowserMode::Browsing,
            Box::new(MainMenu),
            String::from("nand:/"),
        )
    }

    // Opens any version of the browser
    pub fn open_browser(
        mode: BrowserMode,
        exit: Box<dyn ClonableUiPage>,
        mut open_path: String,
    ) -> Option<Browser> {
        fatfs_embedded::opendir(&mut open_path).ok().map(|mut i| {
            let immediate_files = populate_fs_vec(&mut i);
            Browser {
                immediate_files,
                current_path: open_path,
                scroll_offset: 0,
                drag_start: 0,
                hold_timer: 0,
                mode,
                exit,
            }
        })
    }
}
/// The state of a file Browser UI
pub struct Browser {
    /// The files in the current directory.
    immediate_files: Vec<FileEntry>,

    /// The path to the current directory.
    current_path: String,

    /// scroll offset (in pixels) to the browser.
    scroll_offset: i32,

    /// The starting point of a swipe on the touchscreen.
    drag_start: i16,

    /// The number of frames one has held a direction button on the touchscreen
    ///
    /// This is positive for downward direction, and negative for upward direction.
    hold_timer: i16,

    /// The purpose of this browser (see [`BrowserMode`] for details)
    mode: BrowserMode,

    /// The UI this browser returns to once closed.
    exit: Box<dyn ClonableUiPage>,
}
/// The
#[derive(Clone)]
pub enum BrowserMode {
    /// Browsing all files on the SD card, roms are launched when pressed, music plays, directories open, etc.
    Browsing,
    /// Look for a specific type of file, and then do something with it once picked (used in the settings gui)
    Searching(BrowserSearch),
}
#[derive(Clone)]
pub struct BrowserSearch {
    pub filter: &'static [FileType],
    pub goal: &'static dyn Fn(&mut GlobalData, String) -> Option<Box<dyn UiPage>>,
}

impl Browser {
    /// Opens a browser that looks for a specific filetype
    /// Once such a file is picked, the `transform` fn is called containing the path of the picked file.
    /// This then lets you open a new UI if you found something interesting.
    pub fn search_file(
        format: &'static [FileType],
        start: String,
        exit: Box<dyn ClonableUiPage>,
        transform: &'static dyn Fn(&mut GlobalData, String) -> Option<Box<dyn UiPage>>,
    ) -> Option<Browser> {
        Browser::open_browser(
            BrowserMode::Searching(BrowserSearch {
                filter: format,
                goal: transform,
            }),
            exit,
            start,
        )
    }
    /// Open an item in the browser
    fn open_new(&self, file_name: &str) -> Option<Box<dyn UiPage>> {
        let mut new_folder = self.current_path.clone() + file_name + "/";
        if let Ok(mut f) = fatfs_embedded::opendir(&mut new_folder) {
            Some(Box::new(Self {
                immediate_files: populate_fs_vec(&mut f),
                current_path: new_folder,
                scroll_offset: 0,
                drag_start: 0,
                hold_timer: 0,
                exit: self.exit.clone_ui(),
                mode: self.mode.clone(),
            }))
        } else {
            None
        }
    }
    /// Decide to do with a file thats been picked in the [`BrowserMode::Browsing`] mode.
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

        // Find the max scroll ofset we may use
        let max_scroll = ((self.immediate_files.len()) as i32 * ITEM_SPACING) - (ITEM_SPACING * 11);

        // Handle touchscreen swipes
        if let Some(drag) = ui.drag() {
            let new_drag = drag.y - self.drag_start;
            self.drag_start += new_drag;
            self.scroll_offset -= new_drag as i32;
            self.scroll_offset = (self.scroll_offset).min(max_scroll).max(0);
        } else {
            self.drag_start = 0;
        }

        // Get a slice of possibly shown items
        let shown_items = self
            .immediate_files
            .get(((self.scroll_offset / ITEM_SPACING) as usize)..)
            .unwrap_or(&[]);

        // Deal with Page up/down
        let items_visible = (shown_items.len() - 1).clamp(0, 10);
        let mut focus_on = None;
        if ui.input_pressed(Input(Buttons::DIRECTION_RIGHT)) {
            focus_on = Some(items_visible);
        }
        if ui.input_pressed(Input(Buttons::DIRECTION_LEFT)) || !ui.has_focus_anywhere() {
            focus_on = Some(0);
        }

        let in_step = self.scroll_offset % ITEM_SPACING; // pixels away we are from nearest file entry boundary

        // Deal with entry up/down
        if ui.input_down(Input(Buttons::DIRECTION_UP)) {
            self.hold_timer += 1;
            ui.request_repaint();
        } else if ui.input_down(Input(Buttons::DIRECTION_DOWN)) {
            self.hold_timer -= 1;
            ui.request_repaint();
        } else {
            self.hold_timer = 0;
        }

        // Start Ui and show the current path as a heading
        let mut new_state: Option<Box<dyn UiPage>> = None;
        let mut new_folder = None;
        ui.label(&self.current_path);

        // Create two rectangles that mask the top/bottom entries being scrolled out of view
        let rect = micro_imgui::Rect::from_two_pos(
            ui.clip_rect().top_left(),
            ui.clip_rect().top_right() + Vec2::new(0, ITEM_SPACING as _),
        );
        let rect2 = micro_imgui::Rect::from_two_pos(
            SCREEN_RECT.bottom_left() - Vec2::new(0, 5 as _),
            SCREEN_RECT.bottom_right(),
        );

        let color = ui.style().background_color;
        // Offset the first entry on the screen
        ui.add_space((ITEM_SPACING - in_step) as i16);
        let max_items = if in_step == 0 { 11 } else { 12 };
        let mut focus = None;
        // Show all the items
        for (i, item) in shown_items.iter().take(max_items).enumerate() {
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
                    BrowserMode::Browsing => new_state = self.standard_goal(&item, data),
                    BrowserMode::Searching(BrowserSearch { filter, goal }) => {
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
        //Draw masking rectangles
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
                self.scroll_offset = (self.scroll_offset).sub(ITEM_SPACING * 10).max(0);
            }
            if focus_on == Some(10) {
                self.scroll_offset = (self.scroll_offset)
                    .add(ITEM_SPACING * 10)
                    .min(max_scroll)
                    .max(0);
            }
            self.scroll_offset -= in_step;
        }
        // If direction button was just pressed, or been held for over half a second
        if self.hold_timer.abs() == 1 || (self.hold_timer.abs() > 30 && (self.hold_timer & 1 == 0))
        {
            if self.hold_timer.is_negative() {
                if focus == Some(items_visible) {
                    if shown_items.len() >= 12 {
                        self.scroll_offset = self.scroll_offset.saturating_add(ITEM_SPACING);
                    }
                } else {
                    ui.focus_next();
                }
                self.scroll_offset -= in_step;
            } else {
                if focus == Some(0) {
                    if self.scroll_offset > 0 {
                        self.scroll_offset = self.scroll_offset.wrapping_sub(ITEM_SPACING).max(0);
                    }
                } else {
                    ui.focus_prev();
                }
                self.scroll_offset -= in_step;
            }
        }
        // handle pressing B button to back out of the current directory
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
        // Handle opening new directory within self
        if let Some(mut new_folder) = new_folder {
            self.immediate_files = populate_fs_vec(&mut new_folder);
            self.scroll_offset = 0;
        }

        new_state
    }
}

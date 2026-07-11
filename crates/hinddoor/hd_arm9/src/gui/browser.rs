use core::ops::{Add, Sub};

use alloc::{boxed::Box, format, string::String, vec::Vec};
use fatfs_embedded::fatfs::FileOptions;
use micro_imgui_ds::{
    micro_imgui::{self, widgets::button::Button, Backend, Color, Sizing, Vec2},
    Input, SCREEN_RECT,
};
use reboot_lib::{music_modules::mods::MODAsyncLoader, Buttons};

use crate::{
    BACKGROUND_COLOR, COLOR_BOOTABLE, COLOR_MUSIC, FileEntry, FileType, get_extension, gui::{
        AppData, GlobalData, MusicPlaying, frontend::{AppBooter, ClonableUiPage, StreamingWav, UiPage, pop_dir_entry}, main_menu::MainMenu,
    }, populate_fs_vec, stop_mod_file,
};
impl Browser {
    pub fn open_sd() -> Option<Browser> {
        Self::open_browser(Box::new(Browser::standard_goal), Box::new(MainMenu), String::from("sdmc:/"))
    }
    pub fn open_nand() -> Option<Browser> {
        Self::open_browser(Box::new(Browser::standard_goal), Box::new(MainMenu), String::from("nand:/"))
    }
    pub fn open_browser(goal: Box<dyn BrowserGoal>, exit: Box<dyn ClonableUiPage>, mut open_path: String) -> Option<Browser> {
        fatfs_embedded::opendir(&mut open_path).ok().map(|mut i| {
            let immediate_files = populate_fs_vec(&mut i);
            Browser {
                immediate_files,
                current_path: open_path,
                offset: 0,
                drag_start: 0,
                hold_timer: 0,
                goal,
                exit
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
    goal: Box<dyn BrowserGoal>,
    exit: Box<dyn ClonableUiPage>,
}
pub trait BrowserGoal: Fn(&Browser, &FileEntry, &mut GlobalData) -> Option<Box<dyn UiPage>> {
    fn clone_goal(&self) -> Box<dyn BrowserGoal>;
} 
impl<T:Fn(&Browser, &FileEntry, &mut GlobalData) -> Option<Box<dyn UiPage>> + Clone + 'static> BrowserGoal for T {
    fn clone_goal(&self) -> Box<dyn BrowserGoal> {
        Box::new(self.clone())
    }
}
impl Browser {
    pub fn search_file<T: Fn(&mut GlobalData, String) -> Option<Box<dyn UiPage>> + Clone + 'static>(format: &'static [FileType], start: String, exit: Box<dyn ClonableUiPage>, transform: T, ) -> Option<Browser> {
        Browser::open_browser(Browser::look_for_file(format, transform), exit, start)
     
    }
    pub fn look_for_file<T: Fn(&mut GlobalData, String) -> Option<Box<dyn UiPage>> + Clone + 'static>(format: &'static [FileType], transform: T) -> Box<dyn BrowserGoal> {
        let a = move |browser: &Browser, entry: &FileEntry, data: &mut GlobalData| {
            if format.contains(&entry.kind) {
                transform(data, browser.current_path.clone() + &entry.file_name)
            } else if entry.kind == FileType::Dir {
                browser.open_new(&entry.file_name)
            } else {
                None
            }
        };
        Box::new(a)
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
                goal: self.goal.clone_goal(),
                exit: self.exit.clone_ui(),
            }))
        } else {
            None
        }
    }
    fn standard_goal(&self, file: &FileEntry, data: &mut GlobalData) -> Option<Box<dyn UiPage>> {
        let FileEntry { file_name, kind, .. } = file;
        match *kind {
                    FileType::Dir => {
                        self.open_new(file_name)
                    }
                    FileType::Rom => {
                        let path = self.current_path.clone() + file_name;
                        Some(Box::new(AppBooter { path }))
                    }
                    FileType::Mod => {
                        let mut path = self.current_path.clone() + file_name;
                        match fatfs_embedded::open(&mut path, FileOptions::Read) {
                            Ok(module) => {
                                data.loading_mod_file = MusicPlaying::None;
                                data.loading_mod_file =
                                    MusicPlaying::Mod(MODAsyncLoader::new(module));
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

        let shown_items = self.immediate_files
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
                FileType::Rom => data.config.theme.bootable_color,
                FileType::Mod => data.config.theme.asset_color,
                FileType::Wav => data.config.theme.asset_color,
                FileType::Bmp => data.config.theme.asset_color,
                FileType::Ini => data.config.theme.asset_color,
                FileType::Dir => data.config.theme.folder_color,
            };
            let response = ui.add(Button::new(&item.display_name, Sizing::Padded(Vec2::new(248, 8)), color));
            if response.focused() {
                focus = Some(i);
            }
            if Some(i) == focus_on {
                ui.set_focus(&response);
                ui.request_repaint();
            }
            if response.clicked() {
                if let Some(new_stuff) = (self.goal)(&self, item, data) {
                    new_state = Some(new_stuff);
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
        if (self.hold_timer.abs() > 30 && (self.hold_timer & 1 == 0)) || self.hold_timer.abs() == 1 {
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
                if focus == Some(0)  {
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
                new_state = Some(Box::new(MainMenu));
            } else {
                pop_dir_entry(&mut self.current_path);
                match fatfs_embedded::opendir(&mut self.current_path) {
                    Ok(f) => {
                        new_folder = Some(f);
                    }
                    Err(_err) => {
                        new_state = Some(Box::new(super::error::Error::new( format!("Filesystem error!"),
                        )));
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

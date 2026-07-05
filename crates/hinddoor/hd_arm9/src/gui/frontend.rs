use core::{alloc::Layout, ops::{Add, Sub}};

use crate::{
    APP_AREA_START, AppArea, BACKGROUND_COLOR, COLOR_BOOTABLE, COLOR_MUSIC, SCREEN_RECT, boot::{self, read_all}, get_extension, gui::{self, Input}, populate_fs_vec, send_mod_file, stop_mod_file,
};
use alloc::{
    boxed::Box,
    format,
    string::{String, ToString},
    vec::Vec,
};
use common::{blowfish::BFCTX, bootstrap::BOOTINFO_MEM};
use fatfs_embedded::fatfs::{File, FileOptions, RawFileSystem};
use micro_imgui_ds::micro_imgui::{self, widgets::checkbox::Checkbox};
use micro_imgui_ds::micro_imgui::{widgets::button::Button, Backend, Color, Sizing, Vec2};
use reboot_lib::{
    Buttons, VIDEO_HARDWARE, autoboot_info::{BOOT_INFO, UnlaunchParams}, music_modules::mods::MODAsyncLoader, timers::TimerControl,
};
pub enum CurrentUI {
    None,
    Error {
        error_string: String,
    },
    Browsing {
        immediate_files: Vec<(String, String, bool, Color)>,
        file_path: String,
        offset: i32,
        drag_start: i16,
        hold_timer: i16,
    },
    LoadingApp {
        file: fatfs_embedded::fatfs::File,
        file_path: String,
    },
    SpecialThanks,
}
pub struct AppData {
    pub autoboot: Option<(String, &'static UnlaunchParams)>,
    pub current_ui: CurrentUI,
    pub blowfish: BFCTX,
    pub loading_mod_file: Option<MODAsyncLoader>,
    pub streaming_wav: Option<StreamingWav>,
    pub nand_fs: RawFileSystem,
    pub sdmc_fs: RawFileSystem,
    pub config: crate::configuration::Config,
    pub sdio_status: u32,
}
pub struct StreamingWav {
    file: fatfs_embedded::fatfs::File,
    data_start: usize,
    data_len: usize,
    player_head: usize,
    channels: u8,
    bits_per_sample: u8,
    scratch_buffer: *mut [u8],
    frequency: u32,
}
const WAV_BUFFER_LEN: usize = 1024*128;
impl StreamingWav {
    pub fn new(mut file: fatfs_embedded::fatfs::File) -> Option<Self> {
        let mut data_start = 0;
        let mut data_len = 0;
        let len = WAV_BUFFER_LEN;
        let a = Layout::from_size_align(len, 4).ok()?;
        let buffer = unsafe { alloc::alloc::alloc(a) };
        let slice = unsafe { core::slice::from_raw_parts_mut(buffer, len) };
        let mut main_chunk = [0u8; 0x2C];
        read_all(&mut main_chunk, &mut file).ok()?;
        if &main_chunk[..4] != b"RIFF" {
            return None;
        }
        if &main_chunk[8..12] != b"WAVE"{
            return None;
        }
        if &main_chunk[12..16] != b"fmt "{
            return None;
        }
        if &main_chunk[36..40] != b"data"{
            return None;
        }
        data_len = u32::from_le_bytes(main_chunk[40..].first_chunk()?.clone()) as usize;
        data_start = 0x2C;
        let frequency = u32::from_le_bytes(main_chunk[24..].first_chunk()?.clone());
        let bits_per_sample = u16::from_le_bytes(main_chunk[34..].first_chunk()?.clone()) as u8;
        let channels = u16::from_le_bytes(main_chunk[22..].first_chunk()?.clone()) as u8;
        

        Some(Self { file, data_start, data_len, player_head: 0, scratch_buffer: slice, channels, bits_per_sample, frequency })
    }
    pub unsafe fn fetch_new(&mut self, mut count: usize) {
        while count > 0 {
            let break_point = self.player_head % WAV_BUFFER_LEN;
            let slice = &mut (&mut *self.scratch_buffer)[break_point..];
            let cut = slice.len().min(count);
            let final_slice = &mut slice[..cut];
            if read_all(final_slice, &mut self.file).is_ok() {
                self.player_head += final_slice.len();
                count -= final_slice.len();
                for val in final_slice {
                    *val = val.wrapping_add(0x80);
                }
            } else {
                return;
            }
        }
    }
}

pub struct FileEntry {
    filename: String,
    truncated_name: String,
    file_type: FileType,
}
pub enum FileType {}
impl AppData {
    pub fn open_sd() -> Option<CurrentUI> {
        let mut file_path = String::from("sdmc:/");
        fatfs_embedded::opendir(&mut file_path).ok().map(|mut i| {
            let immediate_files = populate_fs_vec(&mut i);
            CurrentUI::Browsing {
                immediate_files,
                file_path,
                offset: 0,
                drag_start: 0,
                hold_timer: 0,
            }
        })
    }
    pub fn open_nand() -> Option<CurrentUI> {
        let mut file_path = String::from("nand:/");
        fatfs_embedded::opendir(&mut file_path).ok().map(|mut i| {
            let immediate_files = populate_fs_vec(&mut i);
            CurrentUI::Browsing {
                immediate_files,
                file_path,
                offset: 0,
                drag_start: 0,
                hold_timer: 0,
            }
        })
    }
    pub unsafe fn autoboot(&mut self) {
        let mut path = core::mem::take(&mut self.config.autoboot);
        let Ok(mut file) = fatfs_embedded::open(&mut path, FileOptions::Read) else {
            return;
        };
        (*(APP_AREA_START as *mut AppArea)).fader.target.write(16);
        //self.current_ui = CurrentUI::LoadingApp { file, file_path: str };
        crate::boot::boot_app(&mut file, &path, self);
    }
    pub fn play_startup_music(&mut self) {
        match fatfs_embedded::open(&mut self.config.style.music, FileOptions::Read) {
            Ok(file) => {
                stop_mod_file();
                self.loading_mod_file = Some(MODAsyncLoader::new(file));
            }
            Err(_abort) => {}
        }
    }
    pub fn update(&mut self, f: &mut micro_imgui::Frame<'_, super::DSMicroGuiBackend>) {
        let _mouse = f.last_known_pointer_location();
        f.central_panel(|ui| {
            {
                let color = BACKGROUND_COLOR;
                ui.paint_shape(micro_imgui::Shape::Rectangle {
                    area: SCREEN_RECT.include_point(Vec2::new(256, 256)),
                    fill: Color(color),
                    rounding: 0,
                    outline_color: Color(color),
                    outline_size: 0,
                });
            }
            if let Some(wav_stream) = &mut self.streaming_wav {
                let pos = unsafe { (*(APP_AREA_START as *mut AppArea)).wav_counter.read()} << 9;
                let bytes_to_read = pos as usize - wav_stream.player_head;
                unsafe { wav_stream.fetch_new(bytes_to_read); };
                unsafe {

                    reboot_lib::flush_mmc();
                    reboot_lib::flush_mmc();
                
                }
                ui.request_repaint();
            }
            if let Some(loading_mod) = self.loading_mod_file.take() {
                let (progress, max) = loading_mod.progress();
                let progress_bar = progress * 30 / max;
                let bar = (0..30)
                    .map(|i| if i < progress_bar { '=' } else { '.' })
                    .collect::<String>();
                ui.label(&format!("Loading [{bar}]"));
                ui.request_repaint();
                match loading_mod.process() {
                    Ok(Some(ret)) => {
                        send_mod_file(ret);
                    }
                    Ok(None) => (),
                    Err(cont) => self.loading_mod_file = Some(cont),
                }
            } else {
                let a = unsafe { (*(APP_AREA_START as *mut AppArea)).wav_counter.read() };
                ui.label(&format!("{a:08x?}"));
            }

            let new_state_fn: Option<Box<dyn FnOnce(CurrentUI) -> CurrentUI>> = match &mut self
                .current_ui
            {
                CurrentUI::Error { error_string } => {
                    ui.header("ERROR:");
                    ui.label(error_string);
                    if ui.button("okay").clicked() {
                        Some(Box::new(|_| CurrentUI::None))
                    } else {
                        None
                    }
                }
                CurrentUI::None => {
                    ui.header("Welcome!");
                    ui.label("Made by Vikrinox, 2026");
                    ui.header(" ");
                    let mut res: Option<Box<dyn FnOnce(CurrentUI) -> CurrentUI>> = None;
                    if ui.button("Browse Files on SD").clicked() {
                        if let Some(sd) = Self::open_sd() {
                            res = Some(Box::new(move |_| sd))
                        }
                    }
                    if ui.button("Browse Files on NAND").clicked() {
                        if let Some(sd) = Self::open_nand() {
                            res = Some(Box::new(move |_| sd))
                        }
                    }
                    ui.add(Checkbox::new(
                        &mut self.config.options.patch_flag,
                        "Enable patching",
                    ));
                    if ui.input_pressed(gui::Input(Buttons::BUTTON_START)) {
                        res = Some(Box::new(|_| CurrentUI::SpecialThanks));
                    }
                    ui.add_space(82);
                    ui.label(concat!("build commit: ", env!("GIT_HASH")));
                    res
                }
                CurrentUI::LoadingApp { file, file_path } => {
                    ui.request_repaint();

                    let mut swap = CurrentUI::None;
                    core::mem::swap(&mut self.current_ui, &mut swap);
                    if let CurrentUI::LoadingApp {
                        mut file,
                        file_path,
                    } = swap
                    {
                        let error = unsafe {
                            (*(APP_AREA_START as *mut AppArea)).fader.target.write(16);
                            boot::boot_app(&mut file, &file_path, self)
                        };
                        unsafe { (*(APP_AREA_START as *mut AppArea)).fader.target.write(0) };
                        let error_string = alloc::format!("Failed to boot file: {error:?}");
                        self.current_ui = CurrentUI::Error { error_string };
                    }

                    None
                }
                CurrentUI::Browsing {
                    immediate_files,
                    file_path: current_path,
                    offset,
                    drag_start,
                    hold_timer,
                } => {
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

                    let mut new_state: Option<
                        alloc::boxed::Box<dyn FnOnce(CurrentUI) -> CurrentUI>,
                    > = None;
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
                                            new_state = Some(Box::new(|_| CurrentUI::LoadingApp {
                                                file,
                                                file_path: bajs,
                                            }));
                                        }
                                        Err(_) => (),
                                    }
                                } else if item.3 == COLOR_MUSIC {
                                    match get_extension(item.1.as_bytes()) {
                                        Some(b".mod") => {                                        
                                            current_path.push_str(&item.1);
                                            match fatfs_embedded::open(current_path, FileOptions::Read) {
                                                Ok(module) => {
                                                    self.loading_mod_file =
                                                        Some(MODAsyncLoader::new(module));
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
                                                        
                                                        unsafe {
                                                            let timer = 0xFFFF - ((33513982 / 2) / wav.frequency) as u16;
                                                            wav.fetch_new(WAV_BUFFER_LEN);
                                                            reboot_lib::flush_mmc();
                                                            reboot_lib::flush_mmc();
                                                            wav.player_head = 0;
                                                            reboot_lib::timers::TIMERS[0].write(reboot_lib::timers::Timer::new(0, TimerControl::empty()));
                                                            (*(APP_AREA_START as *mut AppArea)).wav_counter.write(0);
                                                            reboot_lib::arm9_start_wav_stream(&mut *wav.scratch_buffer, 0, timer);
                                                            reboot_lib::timers::TIMERS[0].write(reboot_lib::timers::Timer::new(timer, TimerControl::ENABLE_IRQ | TimerControl::PRESCALE_1024 | TimerControl::START));

                                                        }
                                                        self.streaming_wav = Some(wav);
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
                    if ui.input_pressed(gui::Input(Buttons::BUTTON_B)) && new_folder.is_none() {
                        if ["nand:/", "sdmc:/"].contains(&current_path.as_str()) {
                            new_state = Some(Box::new(|_| CurrentUI::None));
                        } else {
                            pop_dir_entry(current_path);
                            match fatfs_embedded::opendir(current_path) {
                                Ok(f) => {
                                    new_folder = Some(f);
                                }
                                Err(_err) => {
                                    new_state = Some(Box::new(|_| CurrentUI::Error {
                                        error_string: format!("Filesystem error!"),
                                    }));
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
                CurrentUI::SpecialThanks => {
                    ui.header("Special thanks");
                    let names = &[
                        "edo9300",
                        "nocash",
                        "Team LNH",
                        "f3l1x_10m",
                        "coderkei",
                        "rmc",
                        "PoroCYon",
                        "AntonioND",
                        "and you!",
                    ];
                    for name in names {
                        ui.label(name);
                    }

                    if ui.input_pressed(gui::Input(Buttons::BUTTON_B)) {
                        Some(Box::new(|_| CurrentUI::None))
                    } else {
                        None
                    }
                }
            };
            if let Some(new_state) = new_state_fn {
                let mut current_ui = CurrentUI::None;
                core::mem::swap(&mut current_ui, &mut self.current_ui);
                self.current_ui = new_state(current_ui);
            }
        });
    }
}
fn pop_dir_entry(current_path: &mut String) {
    current_path.pop();
    while current_path.pop() != Some('/') {}
    current_path.push('/');
}

use core::{
    alloc::Layout,
    ops::{Add, Sub},
};

use crate::{
    boot::{self, read_all},
    get_extension,
    gui::{self, Input},
    populate_fs_vec, send_mod_file, stop_mod_file, AppArea, APP_AREA_START, BACKGROUND_COLOR,
    COLOR_BOOTABLE, COLOR_MUSIC, SCREEN_RECT,
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
    autoboot_info::{UnlaunchParams, BOOT_INFO},
    music_modules::mods::MODAsyncLoader,
    sound::SoundControl,
    timers::TimerControl,
    Buttons, VIDEO_HARDWARE,
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
    pub loading_mod_file: MusicPlaying,
    pub nand_fs: RawFileSystem,
    pub sdmc_fs: RawFileSystem,
    pub config: crate::configuration::Config,
    pub sdio_status: u32,
}
pub enum MusicPlaying {
    None,
    Mod(MODAsyncLoader),
    Wav(StreamingWav),
}
pub struct StreamingWav {
    file: fatfs_embedded::fatfs::File,
    data_start: usize,
    data_len: usize,
    player_head: usize,
    scratch_buffer: &'static mut [u8],
    stream_type: StreamType,
    frequency: u32,
}
enum StreamType {
    MonoU8,
    MonoI16,
    StereoU8 { audio: &'static mut [u8] },
    StereoI16 { audio: &'static mut [u8] },
}

const WAV_BUFFER_LEN: usize = 1024 * 64;
const WAV_BUFFER_LAYOUT: Layout = unsafe { Layout::from_size_align_unchecked(WAV_BUFFER_LEN, 4) };
impl Drop for StreamingWav {
    fn drop(&mut self) {
        unsafe { 
            self.stop();
            alloc::alloc::dealloc(self.scratch_buffer.as_mut_ptr(), WAV_BUFFER_LAYOUT) 
        };
    }
}
impl Drop for StreamType {
    fn drop(&mut self) {
        unsafe {
            match self {
                StreamType::MonoU8 => (),
                StreamType::MonoI16 => (),
                StreamType::StereoU8 { audio } => {
                    alloc::alloc::dealloc(audio.as_mut_ptr(), WAV_BUFFER_LAYOUT);
                },
                StreamType::StereoI16 { audio } => {
                    alloc::alloc::dealloc(audio.as_mut_ptr(), WAV_BUFFER_LAYOUT);
                },
            }
        };
    }
}
fn alloc_wav_buf() -> &'static mut [u8] {
    let buffer = unsafe { alloc::alloc::alloc(WAV_BUFFER_LAYOUT) };
    unsafe { core::slice::from_raw_parts_mut(buffer, WAV_BUFFER_LEN) }
}
impl StreamingWav {
    pub fn new(mut file: fatfs_embedded::fatfs::File) -> Option<Self> {
        let mut data_start = 0;
        let mut data_len = 0;

        let mut main_chunk = [0u8; 0x2C];
        read_all(&mut main_chunk, &mut file).ok()?;
        if &main_chunk[..4] != b"RIFF" {
            return None;
        }
        if &main_chunk[8..12] != b"WAVE" {
            return None;
        }
        if &main_chunk[12..16] != b"fmt " {
            return None;
        }
        if &main_chunk[36..40] != b"data" {
            return None;
        }
        data_len = u32::from_le_bytes(main_chunk[40..].first_chunk()?.clone()) as usize;
        data_start = 0x2C;
        let frequency = u32::from_le_bytes(main_chunk[24..].first_chunk()?.clone());
        let bits_per_sample = u16::from_le_bytes(main_chunk[34..].first_chunk()?.clone()) as u8;
        let channels = u16::from_le_bytes(main_chunk[22..].first_chunk()?.clone()) as u8;
        if frequency > 48000 {
            return None;
        }
        let stream = match (channels, bits_per_sample) {
            (1, 8) => StreamType::MonoU8,
            (1, 16) => StreamType::MonoI16,
            (2, 8) => StreamType::StereoU8 {
                audio: alloc_wav_buf(),
            },
            (2, 16) => StreamType::StereoI16 { audio: alloc_wav_buf() },
            _ => return None,
        };
        Some(Self {
            file,
            data_start,
            data_len,
            player_head: 0,
            scratch_buffer: alloc_wav_buf(),
            stream_type: stream,
            frequency,
        })
    }
    pub unsafe fn play(&mut self) {
        let wav = self;
        unsafe {
            let timer = ((33513982 / 2) / wav.frequency) as u16;
            let snd_timer = 0u16.wrapping_sub(timer);
            let (format, timer_timer) = match wav.stream_type {
                StreamType::MonoU8 => (
                    SoundControl::START
                        .with_sound_format(reboot_lib::sound::SoundFormat::PCM8)
                        .with_volume(127)
                        .with_repeat_mode(reboot_lib::sound::RepeatMode::Infinite),
                    0u16.wrapping_sub(timer*4)
                ),
                StreamType::MonoI16 => (
                    SoundControl::START
                        .with_sound_format(reboot_lib::sound::SoundFormat::PCM16)
                        .with_volume(127)
                        .with_repeat_mode(reboot_lib::sound::RepeatMode::Infinite),
                    0u16.wrapping_sub(timer*2)
                    
                ),
                StreamType::StereoU8 { .. } => (
                    SoundControl::START
                        .with_sound_format(reboot_lib::sound::SoundFormat::PCM8)
                        .with_volume(127)
                        .with_repeat_mode(reboot_lib::sound::RepeatMode::Infinite),
                    0u16.wrapping_sub(timer*2)
                ),
                StreamType::StereoI16 { .. } => (
                    SoundControl::START
                        .with_sound_format(reboot_lib::sound::SoundFormat::PCM16)
                        .with_volume(127)
                        .with_repeat_mode(reboot_lib::sound::RepeatMode::Infinite),
                    0u16.wrapping_sub(timer)
                ),                                       
                    
            };
            wav.fetch_new(WAV_BUFFER_LEN);
            wav.player_head = 0;
            reboot_lib::timers::TIMERS[0].write(reboot_lib::timers::Timer::new(0, TimerControl::empty()));
            (*(APP_AREA_START as *mut AppArea)).wav_counter.write(0);
            match &mut wav.stream_type {
                StreamType::MonoU8 => {
                    reboot_lib::arm9_manual_sound_write(wav.scratch_buffer, 0, snd_timer, format.with_panning(0x40), 0);
                },
                StreamType::MonoI16 => {
                    reboot_lib::arm9_manual_sound_write(wav.scratch_buffer, 0, snd_timer, format.with_panning(0x40), 0);
                
                },
                StreamType::StereoU8 { audio } => {
                    let (left,right) = audio.split_at_mut(WAV_BUFFER_LEN/2);
                    reboot_lib::arm9_manual_sound_write(left, 0, snd_timer, format.with_panning(0x0), 0);
                    reboot_lib::arm9_manual_sound_write(right, 1, snd_timer, format.with_panning(0x7F), 0);
                },
                StreamType::StereoI16 { audio } => {
                    let (left,right) = audio.split_at_mut(WAV_BUFFER_LEN/2);
                    reboot_lib::arm9_manual_sound_write(left, 0, snd_timer, format.with_panning(0x0), 0);
                    reboot_lib::arm9_manual_sound_write(right, 1, snd_timer, format.with_panning(0x7F), 0);
                },
            }
            
            
            reboot_lib::timers::TIMERS[0].write(reboot_lib::timers::Timer::new(timer_timer, TimerControl::ENABLE_IRQ | TimerControl::PRESCALE_1024 | TimerControl::START));

        }
    }
    pub unsafe fn stop(&mut self) {
        reboot_lib::timers::TIMERS[0].write(reboot_lib::timers::Timer::new(0, TimerControl::empty()));
        (*(APP_AREA_START as *mut AppArea)).wav_counter.write(0);
        let format = SoundControl::empty();
        match &mut self.stream_type {
            StreamType::MonoU8 => {
                reboot_lib::arm9_manual_sound_write(&mut [], 0, 0, format, 0);
            },
            StreamType::MonoI16 => {
                reboot_lib::arm9_manual_sound_write(&mut [], 0, 0, format, 0);
            
            },
            StreamType::StereoU8 { .. } => {
                reboot_lib::arm9_manual_sound_write(&mut [], 0, 0, format, 0);
                reboot_lib::arm9_manual_sound_write(&mut [], 1, 0, format, 0);
            },
            StreamType::StereoI16 { .. } => {
                reboot_lib::arm9_manual_sound_write(&mut [], 0, 0, format, 0);
                reboot_lib::arm9_manual_sound_write(&mut [], 1, 0, format, 0);
            },
        }
    }
    pub unsafe fn fetch_new(&mut self, mut count: usize) {
        pub fn read_all(
            mut buffer: &mut [u8],
            file: &mut fatfs_embedded::fatfs::File,
            start_point: u32,
        ) -> Result<(), fatfs_embedded::fatfs::Error> {
            while !buffer.is_empty() {
                let bytes = fatfs_embedded::read(file, buffer)?;
                if bytes == 0 {
                    let size = fatfs_embedded::size(file);
                    if size == file.fptr {
                        fatfs_embedded::seek(file, start_point)?;
                    }
                }
                let Some(remaining) = buffer.get_mut((bytes as usize)..) else {
                    return Err(fatfs_embedded::fatfs::Error::InternalLogicError);
                };
                buffer = remaining;
            }
            Ok(())
        }
        while count > 0 {
            let break_point = self.player_head % WAV_BUFFER_LEN;
            let slice = &mut (&mut *self.scratch_buffer)[break_point..];
            let cut = slice.len().min(count);
            let final_slice = &mut slice[..cut];
            if read_all(final_slice, &mut self.file, self.data_start as u32).is_ok() {
                self.player_head += final_slice.len();
                count -= final_slice.len();
                match &mut self.stream_type {
                    StreamType::MonoU8 => {
                        for val in final_slice {
                            *val = val.wrapping_add(0x80);
                        }
                    }
                    StreamType::MonoI16 => (),
                    StreamType::StereoU8 { audio } => {
                        let (left, right) = audio.split_at_mut(WAV_BUFFER_LEN / 2);
                        for (i, val) in final_slice.iter().enumerate() {
                            if i & 1 == 0 {
                                left[(break_point + i) / 2] = val.wrapping_add(0x80);
                            } else {
                                right[(break_point + i) / 2] = val.wrapping_add(0x80)
                            }
                        }
                    }
                    StreamType::StereoI16 { audio } => {
                        let (left, right) = audio.split_at_mut(WAV_BUFFER_LEN / 2);
                        let break_point = break_point/2;
                        for (i, val) in final_slice.chunks_exact(2).enumerate() {
                            if i & 1 == 0 {
                                left[break_point+i] = val[0];
                                left[break_point+i+1] = val[1];
                                
                            } else {
                                right[break_point+i-1] = val[0];
                                right[break_point+i] = val[1];
                            }
                        }
                    },
                }
            } else {
                self.stop();
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
                self.loading_mod_file = MusicPlaying::None;
                let Some(extension) = get_extension(self.config.style.music.as_bytes()) else { return };
                self.loading_mod_file = match extension {
                    b".mod" | b".MOD" => MusicPlaying::Mod(MODAsyncLoader::new(file)),
                    b".wav" | b".WAV" => {
                        if let Some(mut stream) = StreamingWav::new(file) {
                            unsafe { stream.play(); }
                            MusicPlaying::Wav(stream)
                        } else {
                            MusicPlaying::None
                        }
                        
                    },
                    _ => MusicPlaying::None,
                };
                
            }
            Err(_abort) => {}
        }
    }
    pub fn load_wallpaper(&mut self) -> Option<crate::bmp::DecodedBMP> {
        let file = fatfs_embedded::open(&mut self.config.style.top_wallpaper, FileOptions::Read).ok()?;
        crate::bmp::DecodedBMP::from_reader(file)
    }
    pub fn do_background_tasks(&mut self) {
        let mut music = MusicPlaying::None;
        core::mem::swap(&mut music, &mut self.loading_mod_file);
        match music {
            MusicPlaying::None => (),
            MusicPlaying::Wav(mut wav_stream) => {
                let pos = unsafe { (*(APP_AREA_START as *mut AppArea)).wav_counter.read()} << 11;
                let bytes_to_read = pos as usize - wav_stream.player_head;
                unsafe { wav_stream.fetch_new(bytes_to_read); };
                self.loading_mod_file = MusicPlaying::Wav(wav_stream);
            },
            MusicPlaying::Mod(loading_mod) => {
                match loading_mod.process() {
                    Ok(Some(ret)) => {
                        send_mod_file(ret);
                    }
                    Ok(None) => (),
                    Err(cont) => self.loading_mod_file = MusicPlaying::Mod(cont),
                }
            }
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
            
            match &self.loading_mod_file {
                MusicPlaying::Mod(loading_mod) => {
                    let (progress, max) = loading_mod.progress();
                    let progress_bar = progress * 30 / max;
                    let bar = (0..30)
                        .map(|i| if i < progress_bar { '=' } else { '.' })
                        .collect::<String>();
                    ui.label(&format!("Loading [{bar}]"));
                    ui.request_repaint();
                },
                _ => {ui.label(" ");}
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
                                                    self.loading_mod_file = MusicPlaying::None;
                                                    self.loading_mod_file = MusicPlaying::Mod(MODAsyncLoader::new(module));
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
                                                        self.loading_mod_file = MusicPlaying::None;
                                                        unsafe { wav.play() };
                                                        self.loading_mod_file = MusicPlaying::Wav(wav);
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
                        "Kai (coderkei)",
                        "rmc",
                        "folf20",
                        "beta215",
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

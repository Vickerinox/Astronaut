use core::alloc::Layout;

use crate::{
    boot::{self, read_all},
    get_extension, send_mod_file, stop_mod_file, AppArea, FileType, APP_AREA_START,
    BACKGROUND_COLOR, SCREEN_RECT,
};
use alloc::{boxed::Box, format, string::String};
use common::blowfish::BFCTX;
use fatfs_embedded::fatfs::FileOptions;
use micro_imgui_ds::micro_imgui;
use micro_imgui_ds::micro_imgui::{Backend, Color, Vec2};
use reboot_lib::{
    autoboot_info::UnlaunchParams, music_modules::mods::MODAsyncLoader, sound::SoundControl,
    timers::TimerControl,
};
pub struct GlobalData {
    pub autoboot: Option<(String, &'static UnlaunchParams)>,
    pub blowfish: BFCTX,
    pub loading_mod_file: MusicPlaying,
    pub config: crate::configuration::Config,
}

pub struct AppData {
    pub global_data: GlobalData,

    pub current_ui: Box<dyn UiPage>,
}
pub enum MusicPlaying {
    None,
    Mod(MODAsyncLoader),
    Wav(StreamingWav),
}
pub struct StreamingWav {
    file: fatfs_embedded::fatfs::File,
    data_start: usize,
    _data_len: usize,
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
                }
                StreamType::StereoI16 { audio } => {
                    alloc::alloc::dealloc(audio.as_mut_ptr(), WAV_BUFFER_LAYOUT);
                }
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
        let data_start;
        let mut data_len;

        let mut main_chunk = [0u8; 0x24];
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

        let frequency = u32::from_le_bytes(main_chunk[24..].first_chunk()?.clone());
        let bits_per_sample = u16::from_le_bytes(main_chunk[34..].first_chunk()?.clone()) as u8;
        let channels = u16::from_le_bytes(main_chunk[22..].first_chunk()?.clone()) as u8;
        if frequency > 48000 {
            return None;
        }

        let mut chunk_buffer = [0u8; 8];
        loop {
            read_all(&mut chunk_buffer, &mut file).ok()?;
            data_len = u32::from_le_bytes(chunk_buffer[4..].try_into().ok()?);
            if chunk_buffer.first_chunk() == Some(b"data") {
                data_start = file.fptr;
                break;
            } else {
                let seek = file.fptr + data_len;
                fatfs_embedded::seek(&mut file, seek).ok()?;
            }
        }

        let stream_type = match (channels, bits_per_sample) {
            (1, 8) => StreamType::MonoU8,
            (1, 16) => StreamType::MonoI16,
            (2, 8) => StreamType::StereoU8 {
                audio: alloc_wav_buf(),
            },
            (2, 16) => StreamType::StereoI16 {
                audio: alloc_wav_buf(),
            },
            _ => return None,
        };
        Some(Self {
            file,
            data_start: data_start as usize,
            _data_len: data_len as usize,
            player_head: 0,
            scratch_buffer: alloc_wav_buf(),
            stream_type,
            frequency,
        })
    }
    pub unsafe fn play(&mut self) {
        unsafe {
            let timer = ((33513982 / 2) / self.frequency) as u16;
            let snd_timer = 0u16.wrapping_sub(timer);
            let (format, timer_timer) = match self.stream_type {
                StreamType::MonoU8 => (
                    SoundControl::START
                        .with_sound_format(reboot_lib::sound::SoundFormat::PCM8)
                        .with_volume(127)
                        .with_repeat_mode(reboot_lib::sound::RepeatMode::Infinite),
                    0u16.wrapping_sub(timer * 4),
                ),
                StreamType::MonoI16 => (
                    SoundControl::START
                        .with_sound_format(reboot_lib::sound::SoundFormat::PCM16)
                        .with_volume(127)
                        .with_repeat_mode(reboot_lib::sound::RepeatMode::Infinite),
                    0u16.wrapping_sub(timer * 2),
                ),
                StreamType::StereoU8 { .. } => (
                    SoundControl::START
                        .with_sound_format(reboot_lib::sound::SoundFormat::PCM8)
                        .with_volume(127)
                        .with_repeat_mode(reboot_lib::sound::RepeatMode::Infinite),
                    0u16.wrapping_sub(timer * 2),
                ),
                StreamType::StereoI16 { .. } => (
                    SoundControl::START
                        .with_sound_format(reboot_lib::sound::SoundFormat::PCM16)
                        .with_volume(127)
                        .with_repeat_mode(reboot_lib::sound::RepeatMode::Infinite),
                    0u16.wrapping_sub(timer),
                ),
            };
            self.fetch_new(WAV_BUFFER_LEN);
            self.player_head = 0;
            reboot_lib::timers::TIMERS[0]
                .write(reboot_lib::timers::Timer::new(0, TimerControl::empty()));
            (*(APP_AREA_START as *mut AppArea)).wav_counter.write(0);
            match &mut self.stream_type {
                StreamType::MonoU8 => {
                    let _ = reboot_lib::arm9_manual_sound_write(
                        self.scratch_buffer,
                        0,
                        snd_timer,
                        format.with_panning(0x40),
                        0,
                    );
                }
                StreamType::MonoI16 => {
                    let _ = reboot_lib::arm9_manual_sound_write(
                        self.scratch_buffer,
                        0,
                        snd_timer,
                        format.with_panning(0x40),
                        0,
                    );
                }
                StreamType::StereoU8 { audio } => {
                    let (left, right) = audio.split_at_mut(WAV_BUFFER_LEN / 2);
                    let _ = reboot_lib::arm9_manual_sound_write(
                        left,
                        0,
                        snd_timer,
                        format.with_panning(0x0),
                        0,
                    );
                    let _ = reboot_lib::arm9_manual_sound_write(
                        right,
                        1,
                        snd_timer,
                        format.with_panning(0x7F),
                        0,
                    );
                }
                StreamType::StereoI16 { audio } => {
                    let (left, right) = audio.split_at_mut(WAV_BUFFER_LEN / 2);
                    let _ = reboot_lib::arm9_manual_sound_write(
                        left,
                        0,
                        snd_timer,
                        format.with_panning(0x0),
                        0,
                    );
                    let _ = reboot_lib::arm9_manual_sound_write(
                        right,
                        1,
                        snd_timer,
                        format.with_panning(0x7F),
                        0,
                    );
                }
            }

            reboot_lib::timers::TIMERS[0].write(reboot_lib::timers::Timer::new(
                timer_timer,
                TimerControl::ENABLE_IRQ | TimerControl::PRESCALE_1024 | TimerControl::START,
            ));
        }
    }
    pub unsafe fn stop(&mut self) {
        reboot_lib::timers::TIMERS[0]
            .write(reboot_lib::timers::Timer::new(0, TimerControl::empty()));
        (*(APP_AREA_START as *mut AppArea)).wav_counter.write(0);
        let format = SoundControl::empty();
        match &mut self.stream_type {
            StreamType::MonoU8 => {
                let _ = reboot_lib::arm9_manual_sound_write(&mut [], 0, 0, format, 0);
            }
            StreamType::MonoI16 => {
                let _ = reboot_lib::arm9_manual_sound_write(&mut [], 0, 0, format, 0);
            }
            StreamType::StereoU8 { .. } => {
                let _ = reboot_lib::arm9_manual_sound_write(&mut [], 0, 0, format, 0);
                let _ = reboot_lib::arm9_manual_sound_write(&mut [], 1, 0, format, 0);
            }
            StreamType::StereoI16 { .. } => {
                let _ = reboot_lib::arm9_manual_sound_write(&mut [], 0, 0, format, 0);
                let _ = reboot_lib::arm9_manual_sound_write(&mut [], 1, 0, format, 0);
            }
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
                        let break_point = break_point / 2;
                        for (i, val) in final_slice.chunks_exact(2).enumerate() {
                            if i & 1 == 0 {
                                left[break_point + i] = val[0];
                                left[break_point + i + 1] = val[1];
                            } else {
                                right[break_point + i - 1] = val[0];
                                right[break_point + i] = val[1];
                            }
                        }
                    }
                }
            } else {
                self.stop();
                return;
            }
        }
    }
}

pub struct AppBooter {
    pub path: String,
}

impl UiPage for AppBooter {
    fn ui(
        &mut self,
        ui: &mut micro_imgui::Ui<'_, '_, micro_imgui_ds::DSMicroGuiBackend>,
        data: &mut GlobalData,
    ) -> Option<Box<dyn UiPage>> {
        let Ok(mut file) = fatfs_embedded::open(&mut self.path, FileOptions::Read) else {
            return Some(Box::new(super::error::Error {
                error_string: format!("File doesn't exist."),
            }));
        };
        ui.request_repaint();
        let error = unsafe {
            (*(APP_AREA_START as *mut AppArea)).fader.target.write(16);
            boot::boot_app(&mut file, &self.path, data)
        };
        unsafe { (*(APP_AREA_START as *mut AppArea)).fader.target.write(0) };
        let error_string = alloc::format!("Failed to boot file: {error:?}");
        Some(Box::new(super::error::Error { error_string }))
    }
}
pub trait ClonableUiPage: UiPage {
    fn clone_ui(&self) -> Box<dyn ClonableUiPage>;
}
pub trait UiPage {
    fn ui(
        &mut self,
        ui: &mut micro_imgui::Ui<'_, '_, micro_imgui_ds::DSMicroGuiBackend>,
        data: &mut GlobalData,
    ) -> Option<Box<dyn UiPage>>;
}
impl<T: Clone + UiPage + 'static> ClonableUiPage for T {
    fn clone_ui(&self) -> Box<dyn ClonableUiPage> {
        Box::new(self.clone())
    }
} 

impl AppData {
    pub unsafe fn autoboot(&mut self) {
        let mut path = core::mem::take(&mut self.global_data.config.autoboot);
        let Ok(mut file) = fatfs_embedded::open(&mut path, FileOptions::Read) else {
            return;
        };
        (*(APP_AREA_START as *mut AppArea)).fader.target.write(16);
        //self.current_ui = CurrentUI::LoadingApp { file, file_path: str };
        crate::boot::boot_app(&mut file, &path, &mut self.global_data);
    }
    pub fn do_background_tasks(&mut self) {
        match &mut self.global_data.loading_mod_file {
            MusicPlaying::None => (),
            MusicPlaying::Wav(wav_stream) => {
                let pos = unsafe { (*(APP_AREA_START as *mut AppArea)).wav_counter.read() } << 11;
                let bytes_to_read = pos as usize - wav_stream.player_head;
                unsafe {
                    wav_stream.fetch_new(bytes_to_read);
                };
            }
            MusicPlaying::Mod(loading_mod) => match loading_mod.process() {
                Some(ret) => {
                    send_mod_file(ret);
                }
                None => if loading_mod.done() {
                    self.global_data.loading_mod_file = MusicPlaying::None;
                },
            },
        }
    }
    pub fn update(&mut self, f: &mut micro_imgui::Frame<'_, super::DSMicroGuiBackend>) {
        let _mouse = f.last_known_pointer_location();
        f.central_panel(|ui| {
            {
                let color = ui.style().background_color;
                ui.paint_shape(micro_imgui::Shape::Rectangle {
                    area: SCREEN_RECT,
                    fill: color,
                    rounding: 0,
                    outline_color: color,
                    outline_size: 0,
                });
            }

            match &self.global_data.loading_mod_file {
                MusicPlaying::Mod(loading_mod) => {
                    let (progress, max) = loading_mod.progress();
                    let progress_bar = progress * 30 / max;
                    let bar = (0..30)
                        .map(|i| if i < progress_bar { '=' } else { '.' })
                        .collect::<String>();
                    ui.label(&format!("Loading [{bar}]"));
                    ui.request_repaint();
                }
                _ => {
                    ui.label(" ");
                }
            }
            if let Some(new_ui) = self.current_ui.ui(ui, &mut self.global_data) {
                self.current_ui = new_ui;
                ui.clear_focus();
            }
        });
    }
}
pub fn pop_dir_entry(current_path: &mut String) {
    current_path.pop();
    while current_path.pop() != Some('/') {}
    current_path.push('/');
}

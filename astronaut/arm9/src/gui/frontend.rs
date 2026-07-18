// SPDX-FileCopyrightText: 2026 Viktor Karlsson <viktor@koda.re>
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{
    boot::{self},
    configuration::Theme,
    music::{send_mod_file, MusicPlaying},
    AppArea, APP_AREA_START, SCREEN_RECT,
};

use alloc::{boxed::Box, format, string::String};
use common::blowfish::BFCTX;
use fatfs_embedded::fatfs::FileOptions;
use micro_imgui_ds::micro_imgui;
use micro_imgui_ds::micro_imgui::Backend;
use reboot_lib::autoboot_info::UnlaunchParams;
use reboot_lib::fatfs_embedded;
pub struct GlobalData {
    pub our_path: String,
    pub blowfish: BFCTX,
    pub loading_mod_file: MusicPlaying,
    pub config: crate::configuration::Config,
    pub theme: Theme,
}

pub struct AppData {
    pub global_data: GlobalData,
    pub current_ui: Box<dyn UiPage>,
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
            return Some(Box::new(super::error::Error::new(format!(
                "File doesn't exist."
            ))));
        };
        ui.request_repaint();
        let error = unsafe {
            (*(APP_AREA_START as *mut AppArea)).fader.target.write(16);
            boot::boot_app(&mut file, &self.path, data)
        };
        unsafe { (*(APP_AREA_START as *mut AppArea)).fader.target.write(0) };
        let error_string = alloc::format!("Failed to boot file: {error:?}");
        Some(Box::new(super::error::Error::new(error_string)))
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
    #[no_mangle]
    #[link_section = ".text_aux"]
    pub fn do_background_tasks(&mut self) {
        match &mut self.global_data.loading_mod_file {
            MusicPlaying::None => (),
            MusicPlaying::Wav(wav_stream) => {
                let pos = unsafe { (*(APP_AREA_START as *mut AppArea)).wav_counter.read() } << 11;
                let bytes_to_read = pos as usize - wav_stream.counter();
                unsafe {
                    wav_stream.fetch_new(bytes_to_read);
                };
            }
            MusicPlaying::Mod(loading_mod) => match loading_mod.process() {
                Some(ret) => {
                    send_mod_file(ret);
                }
                None => {
                    if loading_mod.done() {
                        self.global_data.loading_mod_file = MusicPlaying::None;
                    }
                }
            },
        }
    }
    #[no_mangle]
    #[link_section = ".text_aux"]
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

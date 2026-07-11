use alloc::{boxed::Box, string::String};
use micro_imgui_ds::{
    Input, micro_imgui::{Backend, InputEvent, widgets::{button::Button, checkbox::Checkbox}},
};
use reboot_lib::Buttons;

use crate::gui::{AppData, MainMenu, browser::Browser, frontend::UiPage};

#[derive(Clone)]
pub struct Settings;

impl UiPage for Settings {
    fn ui(
        &mut self,
        ui: &mut micro_imgui_ds::micro_imgui::Ui<'_, '_, micro_imgui_ds::DSMicroGuiBackend>,
        data: &mut super::GlobalData,
    ) -> Option<alloc::boxed::Box<dyn UiPage>> {
        let mut result: Option<Box<dyn UiPage>> = None;
        if ui.input_pressed(Input::FOCUS_NEXT) || !ui.has_focus_anywhere() {
            ui.focus_next();
        } else if ui.input_pressed(Input::FOCUS_PREVIOUS) {
            ui.focus_prev();
        }
        ui.header("Settings");
        
        ui.add_space(8);
        ui.label("Boot Options:");
        ui.add(Checkbox::new(
            &mut data.config.patch_flag,
            "DSi Menu patching",
        ));
        ui.add(Checkbox::new(
            &mut data.config.wifi_firmware_upload,
            "WiFi Firmware upload",
        ));

        ui.add_space(4);
        ui.label("Theme:");
        if ui.button(data.config.theme_path.is_empty().then_some("(none)").unwrap_or(&data.config.theme_path)).clicked() {
            let b = AppData::open_browser(Browser::look_for_file(&[crate::FileType::Ini], &|data, path| { data.config.theme_path = path; Some(Box::new(Self))}), Box::new(Self), String::from("sdmc:/"));
            if let Some(b) = b {
                result = Some(Box::new(b))
            }
        }


        ui.add_space(4);
        ui.label("Override Music:");
        if ui.button(data.config.music.is_empty().then_some("(none)").unwrap_or(&data.config.music)).clicked() {
            let b = AppData::open_browser(Browser::look_for_file(&[crate::FileType::Wav, crate::FileType::Mod], &|data, path| { data.config.music = path; Some(Box::new(Self))}), Box::new(Self), String::from("sdmc:/"));
            if let Some(b) = b {
                result = Some(Box::new(b))
            }
        }

        ui.add_space(4);
        ui.label("Override Wallpaper:");
        if ui.button(data.config.top_wallpaper.is_empty().then_some("(none)").unwrap_or(&data.config.top_wallpaper)).clicked() {
            let b = AppData::open_browser(Browser::look_for_file(&[crate::FileType::Wav, crate::FileType::Mod], &|data, path| { data.config.top_wallpaper = path; Some(Box::new(Self))}), Box::new(Self), String::from("sdmc:/"));
            if let Some(b) = b {
                result = Some(Box::new(b))
            }
        }
        ui.add_space(ui.clip_rect().height()-14);
        ui.horizontal(|ui| {
            if ui.button("go back").clicked() {
                result = Some(Box::new(MainMenu));
            }
            if ui.button("save").clicked() {
                result = Some(Box::new(MainMenu));
            }  
        });
        
        result
    }
}

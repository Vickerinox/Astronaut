use alloc::{boxed::Box, format, string::{String, ToString}};
use fatfs_embedded::fatfs::FileOptions;
use micro_imgui_ds::{
    Input, micro_imgui::{Backend, InputEvent, widgets::{button::Button, checkbox::Checkbox}},
};
use reboot_lib::Buttons;

use crate::{FileType, configuration::BootCombo, gui::{AppData, GlobalData, MainMenu, browser::Browser, frontend::UiPage}};

#[derive(Clone)]
pub enum Settings {
    Main,
    BootCombos,
    SelectedCombo(Buttons, u32),
}
impl Settings {
    fn main_settings(&mut self, ui: &mut micro_imgui_ds::micro_imgui::Ui<'_, '_, micro_imgui_ds::DSMicroGuiBackend>,
    data: &mut super::GlobalData) -> Option<Box<dyn UiPage>> {
        let mut result: Option<Box<dyn UiPage>> = None;
        
        ui.header("Settings");
        
        ui.add_space(8);
        ui.label("Boot Options:");
        if ui.button("Change Boot Combos").clicked() {
            *self = Self::BootCombos;
        }
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
        ui.horizontal(|ui|{
            if ui.button("reset").clicked() {
                data.config.theme_path = String::new();
            }
            if ui.button(data.config.theme_path.is_empty().then_some("(none)").unwrap_or(&data.config.theme_path)).clicked() {
                let b = Browser::search_file(&[FileType::Ini], String::from("sdmc:/"), Box::new(Self::Main), &|data: &mut GlobalData, path: String| -> Option<Box<dyn UiPage>> { data.config.theme_path = path; Some(Box::new(Self::Main))});
                if let Some(b) = b {
                    result = Some(Box::new(b))
                }
            }
        });
        


        ui.add_space(4);
        ui.label("Override Music:");
        ui.horizontal(|ui| {
            if ui.button("reset").clicked() {
                data.config.music = String::new();
            }
            if ui.button(data.config.music.is_empty().then_some("(none)").unwrap_or(&data.config.music)).clicked() {
                let b = Browser::search_file(&[FileType::Wav, FileType::Mod], String::from("sdmc:/"), Box::new(Self::Main), &|data: &mut GlobalData, path: String| -> Option<Box<dyn UiPage>> { data.config.music = path; Some(Box::new(Self::Main))});
                if let Some(b) = b {
                    result = Some(Box::new(b))
                }
            }
        });

        ui.add_space(4);
        
        ui.label("Override Wallpaper:");
        ui.horizontal(|ui| {
            if ui.button("reset").clicked() {
                data.config.top_wallpaper = String::new();
            }
            if ui.button(data.config.top_wallpaper.is_empty().then_some("(none)").unwrap_or(&data.config.top_wallpaper)).clicked() {
                let b = Browser::search_file(&[FileType::Wav, FileType::Mod], String::from("sdmc:/"), Box::new(Self::Main), &|data: &mut GlobalData, path: String| -> Option<Box<dyn UiPage>> { data.config.top_wallpaper = path; Some(Box::new(Self::Main))});
                if let Some(b) = b {
                    result = Some(Box::new(b))
                }
            }
        });
        
        
        ui.add_space(ui.clip_rect().height()-14);
        ui.horizontal(|ui| {
            if ui.button("exit").clicked() {
                result = Some(Box::new(MainMenu));
            }
            if ui.button("save").clicked() {
                result = Some(Box::new(super::error::Error::new(String::from("FAILED TO WRITE NEW CONFIG"))));
                if let Ok(mut file) = fatfs_embedded::open(&mut "sdmc:/_nds/vlaunch/settings.ini".to_string(), FileOptions::Write) {
                    
                    let new_ini = data.config.into_ini();
                    let bytes = new_ini.as_bytes();
                    let Ok(stuff) = fatfs_embedded::write(&mut file, bytes) else { return result;};
                    if stuff != bytes.len() as _ {
                        return result;
                    }
                    if fatfs_embedded::truncate(&mut file).is_err() {
                        return result;
                    }
                    result = Some(Box::new(MainMenu));
                }
                
            }    
            result
        })
    }
    fn boot_combo_settings(&mut self, ui: &mut micro_imgui_ds::micro_imgui::Ui<'_, '_, micro_imgui_ds::DSMicroGuiBackend>,
    data: &mut super::GlobalData) -> Option<Box<dyn UiPage>> {

        ui.label("default boot option:");
        if ui.button(data.config.boot_combos.default.is_empty().then_some("(none)").unwrap_or(&data.config.boot_combos.default)).clicked() {
            *self = Self::SelectedCombo(Buttons::empty(), 999)
        }
        let mut delete = None;
        for (i,j) in data.config.boot_combos.additionals.iter_mut().enumerate().take(5) {
            ui.add_space(4);
            let BootCombo { buttons, path } = j;
            ui.label(&format!("Combo {}:", format_combo(*buttons)));
            ui.horizontal(|ui| {
                if ui.button("delete").clicked() {
                delete = Some(i);
            }
            if ui.button(path).clicked() {
                *self = Self::SelectedCombo(*buttons, 999)
            }
        
            })
            
            

        }
        if let Some(del) = delete {
            data.config.boot_combos.additionals.remove(del);
        }
        ui.add_space(ui.clip_rect().height()-14);
        ui.horizontal(|ui| {
            if ui.button("go back").clicked() {
                *self = Self::Main;
            }    
            if ui.button("new combo").clicked() {
                *self = Self::SelectedCombo(Buttons::empty(), 0);
            }
        });
        
        None
    }
}
impl UiPage for Settings {
    fn ui(
        &mut self,
        ui: &mut micro_imgui_ds::micro_imgui::Ui<'_, '_, micro_imgui_ds::DSMicroGuiBackend>,
        data: &mut super::GlobalData,
    ) -> Option<Box<dyn UiPage>> {
        if ui.input_pressed(Input::FOCUS_NEXT) || (!ui.has_focus_anywhere() && ui.backend().held_buttons().is_empty()) {
            ui.focus_next();
        } else if ui.input_pressed(Input::FOCUS_PREVIOUS) {
            ui.focus_prev();
        }
        match self {
            Settings::Main => self.main_settings(ui, data),
            Settings::BootCombos => self.boot_combo_settings(ui, data),
            Settings::SelectedCombo(combo, timer) => {
                let buttons = ui.backend().held_buttons();
                if *timer > 0 {
                    if *timer > 90 {
                        if *combo == Buttons::BUTTON_A | Buttons::BUTTON_B {
                            *self = Self::BootCombos;
                            ui.request_repaint();
                            None
                        } else {
                            let mut ret: Option<Box<dyn UiPage>> = None;
                            ui.label(&format!("you've chosen: {}", format_combo(*combo)));
                            let buttons = *combo;
                            let a = move |data: &mut GlobalData, path: String| -> Option<Box<dyn UiPage>> { 
                                if buttons.is_empty() {
                                    data.config.boot_combos.set_default(path); 
                                
                                } else {
                                    data.config.boot_combos.add(BootCombo {buttons, path}); 
                                }
                                Some(Box::new(Settings::BootCombos))
                            };
                            if ui.button("Launch something from SD").clicked() {
                                
                                let b = Browser::search_file(&[FileType::Rom], String::from("sdmc:/"), Box::new(Self::BootCombos), Box::new(a));
                                if let Some(b) = b {
                                    ret = Some(Box::new(b))
                                }
                            }
                            if ui.button("Launch something from NAND").clicked() {
                            
                                let b = Browser::search_file(&[FileType::Rom], String::from("nand:/"), Box::new(Self::BootCombos), Box::new(a));
                                if let Some(b) = b {
                                    ret = Some(Box::new(b))
                                }
                            }
                            if ui.button("cancel").clicked() {
                                *self = Self::BootCombos;
                            }
                            ret
                        }
                    } else {
                        ui.label(&format_combo(buttons));
                        if buttons != *combo {
                            *combo = buttons;
                            *timer = 1;    
                        } else if buttons == Buttons::empty() {
                            *timer = 0;
                        }else {
                            *timer += 1;
                        }
                        ui.request_repaint();
                        None
                    }
                } else {
                    
                    if !buttons.is_empty() {
                        *combo = buttons;
                        *timer = 1;
                        ui.request_repaint();
                    }
                    ui.label("hold a button combo to start, or A+B to cancel.");
                    None
                }
            },
        }
    }
}

fn format_combo(buttons: Buttons) -> String {
    if buttons.is_empty() {
        return String::from("default");
    }
    let mut string = String::new();
    if buttons.contains(Buttons::BUTTON_A) {
        string += "A+";
    }
    if buttons.contains(Buttons::BUTTON_B) {
        string += "B+";
    }
    if buttons.contains(Buttons::BUTTON_X) {
        string += "X+";
    }
    if buttons.contains(Buttons::BUTTON_Y) {
        string += "Y+";
    }
    if buttons.contains(Buttons::BUTTON_L) {
        string += "L+";
    }
    if buttons.contains(Buttons::BUTTON_R) {
        string += "R+";
    }
    if buttons.contains(Buttons::BUTTON_START) {
        string += "Start+";
    }
    if buttons.contains(Buttons::BUTTON_SELECT) {
        string += "Select+";
    }
    if buttons.contains(Buttons::DIRECTION_UP) {
        string += "Up+";
    }
    if buttons.contains(Buttons::DIRECTION_DOWN) {
        string += "Down+";
    }
    if buttons.contains(Buttons::DIRECTION_LEFT) {
        string += "Left+";
    }
    if buttons.contains(Buttons::DIRECTION_RIGHT) {
        string += "Right+";
    }
    string.pop();
    string
}

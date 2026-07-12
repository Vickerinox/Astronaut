use alloc::{
    boxed::Box,
    format,
    string::{String, ToString},
};
use fatfs_embedded::fatfs::FileOptions;
use micro_imgui_ds::{
    micro_imgui::{
        widgets::{button::Button, checkbox::Checkbox},
        Backend, InputEvent, Response,
    },
    Input,
};
use reboot_lib::Buttons;

use crate::{
    configuration::BootCombo,
    gui::{browser::Browser, frontend::UiPage, AppData, GlobalData, MainMenu},
    truncate_name, FileType,
};

#[derive(Clone)]
pub enum Settings {
    Main,
    BootCombos(usize),
    SelectedCombo(Buttons, u32),
}
impl Settings {
    fn main_settings(
        &mut self,
        ui: &mut micro_imgui_ds::micro_imgui::Ui<'_, '_, micro_imgui_ds::DSMicroGuiBackend>,
        data: &mut super::GlobalData,
    ) -> Option<Box<dyn UiPage>> {
        let mut result: Option<Box<dyn UiPage>> = None;

        ui.header("Settings");

        ui.add_space(8);
        ui.label("Boot Options:");
        if ui.button("Change Boot Combos").clicked() {
            *self = Self::BootCombos(0);
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
        ui.horizontal(|ui| {
            if ui.button("reset").clicked() {
                data.config.theme_path = String::new();
            }
            if path_button(ui, &data.config.theme_path, 28).clicked() {
                let b = Browser::search_file(
                    &[FileType::Ini],
                    String::from("sdmc:/"),
                    Box::new(Self::Main),
                    Box::new(
                        |data: &mut GlobalData, path: String| -> Option<Box<dyn UiPage>> {
                            data.config.theme_path = path;
                            Some(Box::new(Self::Main))
                        },
                    ),
                );
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
            if path_button(ui, &data.config.music, 28).clicked() {
                let b = Browser::search_file(
                    &[FileType::Wav, FileType::Mod],
                    String::from("sdmc:/"),
                    Box::new(Self::Main),
                    Box::new(
                        |data: &mut GlobalData, path: String| -> Option<Box<dyn UiPage>> {
                            data.config.music = path;
                            Some(Box::new(Self::Main))
                        },
                    ),
                );
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
            if path_button(ui, &data.config.top_wallpaper, 28).clicked() {
                let b = Browser::search_file(
                    &[FileType::Wav, FileType::Mod],
                    String::from("sdmc:/"),
                    Box::new(Self::Main),
                    Box::new(
                        |data: &mut GlobalData, path: String| -> Option<Box<dyn UiPage>> {
                            data.config.top_wallpaper = path;
                            Some(Box::new(Self::Main))
                        },
                    ),
                );
                if let Some(b) = b {
                    result = Some(Box::new(b))
                }
            }
        });

        ui.add_space(ui.clip_rect().height() - 14);
        ui.horizontal(|ui| {
            if ui.button("exit").clicked() {
                result = Some(Box::new(MainMenu));
            }
            if ui.button("save").clicked() {
                result = Some(Box::new(super::error::Error::new(String::from(
                    "FAILED TO WRITE NEW CONFIG",
                ))));
                if let Ok(mut file) = fatfs_embedded::open(
                    &mut "sdmc:/_nds/vlaunch/settings.ini".to_string(),
                    FileOptions::Write | FileOptions::CreateAlways,
                ) {
                    let new_ini = data.config.into_ini();
                    let bytes = new_ini.as_bytes();
                    let Ok(stuff) = fatfs_embedded::write(&mut file, bytes) else {
                        return result;
                    };
                    if stuff != bytes.len() as _ {
                        return result;
                    }
                    result = Some(Box::new(MainMenu));
                }
            }
            result
        })
    }
    fn boot_combo_settings(
        &mut self,
        page: usize,
        ui: &mut micro_imgui_ds::micro_imgui::Ui<'_, '_, micro_imgui_ds::DSMicroGuiBackend>,
        data: &mut super::GlobalData,
    ) -> Option<Box<dyn UiPage>> {
        const PAGE_SIZE: usize = 4;
        ui.label("default boot option:");
        if path_button(ui, &data.config.boot_combos.default, 35).clicked()
        {
            *self = Self::SelectedCombo(Buttons::empty(), 999)
        }
        let mut delete = None;
        let total_pages = data
            .config
            .boot_combos
            .additionals
            .is_empty()
            .then_some(0)
            .unwrap_or((data.config.boot_combos.additionals.len() - 1) / PAGE_SIZE);

        let show_additionals = data
            .config
            .boot_combos
            .additionals
            .get_mut(page * PAGE_SIZE..)
            .unwrap_or(&mut []);
        ui.add_space(8);

        for (i, j) in show_additionals.iter_mut().enumerate().take(PAGE_SIZE) {
            ui.add_space(4);
            let BootCombo { buttons, path } = j;
            ui.label(&format!("Combo {}:", format_combo(*buttons)));
            ui.horizontal(|ui| {
                if ui.button("delete").clicked() {
                    delete = Some(i);
                }
                if path_button(ui, path, 28).clicked() {
                    *self = Self::SelectedCombo(*buttons, 999)
                }
            })
        }
        ui.add_space(ui.clip_rect().height() - 24);
        ui.label(&format!(
            "page {}/{} (l or r dpad/buttons)",
            page + 1,
            total_pages + 1
        ));
        ui.add_space(2);

        if let Some(del) = delete {
            data.config.boot_combos.additionals.remove(del);
        }
        if page < total_pages {
            if ui.input_pressed(Input(Buttons::BUTTON_R))
                || ui.input_pressed(Input(Buttons::DIRECTION_RIGHT))
            {
                *self = Self::BootCombos(page + 1)
            }
        }
        if page > 0 {
            if ui.input_pressed(Input(Buttons::BUTTON_L))
                || ui.input_pressed(Input(Buttons::DIRECTION_LEFT))
            {
                *self = Self::BootCombos(page - 1)
            }
        }
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
fn path_button(
    ui: &mut micro_imgui_ds::micro_imgui::Ui<'_, '_, micro_imgui_ds::DSMicroGuiBackend>,
    text: &str,
    limit: usize,
) -> Response {
    if text.is_empty() {
        ui.button("(none)")
    } else {
        ui.button(&truncate_name(&text, limit))
    }
}
impl UiPage for Settings {
    fn ui(
        &mut self,
        ui: &mut micro_imgui_ds::micro_imgui::Ui<'_, '_, micro_imgui_ds::DSMicroGuiBackend>,
        data: &mut super::GlobalData,
    ) -> Option<Box<dyn UiPage>> {
        crate::focus_default(ui);
        match self {
            Settings::Main => self.main_settings(ui, data),
            Settings::BootCombos(page) => {
                let a = *page;
                self.boot_combo_settings(a, ui, data)
            }
            Settings::SelectedCombo(combo, timer) => {
                let buttons = ui.backend().held_buttons();
                if *timer > 0 {
                    if *timer > 90 {
                        if *combo == Buttons::BUTTON_A | Buttons::BUTTON_B {
                            *self = Self::BootCombos(0);
                            ui.request_repaint();
                            None
                        } else {
                            let mut ret: Option<Box<dyn UiPage>> = None;
                            ui.label(&format!("you've chosen: {}", format_combo(*combo)));
                            let buttons = *combo;
                            let a = move |data: &mut GlobalData,
                                          path: String|
                                  -> Option<Box<dyn UiPage>> {
                                if buttons.is_empty() {
                                    data.config.boot_combos.set_default(path);
                                } else {
                                    data.config.boot_combos.add(BootCombo { buttons, path });
                                }
                                Some(Box::new(Settings::BootCombos(0)))
                            };
                            if ui.button("Launch something from SD").clicked() {
                                let b = Browser::search_file(
                                    &[FileType::Rom],
                                    String::from("sdmc:/"),
                                    Box::new(Self::BootCombos(0)),
                                    Box::new(a),
                                );
                                if let Some(b) = b {
                                    ret = Some(Box::new(b))
                                }
                            }
                            if ui.button("Launch something from NAND").clicked() {
                                let b = Browser::search_file(
                                    &[FileType::Rom],
                                    String::from("nand:/"),
                                    Box::new(Self::BootCombos(0)),
                                    Box::new(a),
                                );
                                if let Some(b) = b {
                                    ret = Some(Box::new(b))
                                }
                            }
                            if ui.button("cancel").clicked() {
                                *self = Self::BootCombos(0);
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
                        } else {
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
            }
        }
    }
}

fn format_combo(buttons: Buttons) -> String {
    if buttons.is_empty() {
        return String::from("default");
    }
    let mut string = String::new();
    const FORMAT_COMBOS: &[(Buttons, &str)] = &[
        (Buttons::BUTTON_A, "A+"),
        (Buttons::BUTTON_B, "B+"),
        (Buttons::BUTTON_X, "X+"),
        (Buttons::BUTTON_Y, "Y+"),
        (Buttons::BUTTON_L, "L+"),
        (Buttons::BUTTON_R, "R+"),
        (Buttons::BUTTON_START, "Start+"),
        (Buttons::BUTTON_SELECT, "Select+"),
        (Buttons::DIRECTION_UP, "Up+"),
        (Buttons::DIRECTION_DOWN, "Down+"),
        (Buttons::DIRECTION_LEFT, "Left+"),
        (Buttons::DIRECTION_RIGHT, "Right+"),
    ];
    for (button, str) in FORMAT_COMBOS {
        if buttons.contains(*button) {
            string += str;
        }
    }
    string.pop();
    string
}

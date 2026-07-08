use alloc::boxed::Box;
use micro_imgui_ds::{gui, micro_imgui::widgets::checkbox::Checkbox};
use reboot_lib::Buttons;

use crate::gui::{AppData, CurrentFrontend, frontend::UiPage, special_thanks::SpecialThanks};

pub struct MainMenu;

impl UiPage for MainMenu {
    fn ui(&mut self, ui: &mut micro_imgui_ds::micro_imgui::Ui<'_, '_, micro_imgui_ds::DSMicroGuiBackend>, data: &mut super::GlobalData) -> Option<Box<dyn UiPage>> {
        ui.header("Welcome!");
        ui.label("Made by Vikrinox, 2026");
        ui.header(" ");
        let mut res: Option<Box<dyn UiPage>> = None;
        if ui.button("Browse Files on SD").clicked() {
            if let Some(sd) = AppData::open_sd() {
                res = Some(Box::new(sd))
            }
        }
        if ui.button("Browse Files on NAND").clicked() {
            if let Some(sd) = AppData::open_nand() {
                res = Some(Box::new(sd))
            }
        }
        ui.add(Checkbox::new(
            &mut data.config.options.patch_flag,
            "Enable patching",
        ));
        if ui.input_pressed(gui::Input(Buttons::BUTTON_START)) {
            res = Some(Box::new(SpecialThanks));
        }
        ui.add_space(82);
        ui.label(concat!("build commit: ", env!("GIT_HASH")));
        res
    }
}
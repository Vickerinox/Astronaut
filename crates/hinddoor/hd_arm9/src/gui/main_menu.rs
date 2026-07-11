use alloc::boxed::Box;
use micro_imgui_ds::{
    Input, gui, micro_imgui::{Backend, InputEvent, widgets::checkbox::Checkbox},
};
use reboot_lib::Buttons;

use crate::gui::{frontend::UiPage, special_thanks::SpecialThanks, AppData};

pub struct MainMenu;

impl UiPage for MainMenu {
    fn ui(
        &mut self,
        ui: &mut micro_imgui_ds::micro_imgui::Ui<'_, '_, micro_imgui_ds::DSMicroGuiBackend>,
        data: &mut super::GlobalData,
    ) -> Option<Box<dyn UiPage>> {
        if ui.input_pressed(Input::FOCUS_NEXT) || !ui.has_focus_anywhere() {
            ui.focus_next();
        } else if ui.input_pressed(Input::FOCUS_PREVIOUS) {
            ui.focus_prev();
        }
        //ui.vertical_centered(|ui| {
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
        if ui.button("Settings").clicked() {
            res = Some(Box::new(super::settings::Settings));
        }
        if ui.input_pressed(gui::Input(Buttons::BUTTON_START)) {
            res = Some(Box::new(SpecialThanks));
        }
        ui.add_space(82);
        ui.label(concat!("build commit: ", env!("GIT_HASH")));
        res
        //})
    }
}

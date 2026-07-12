use alloc::boxed::Box;
use micro_imgui_ds::{
    gui,
    micro_imgui::{widgets::checkbox::Checkbox, Backend, InputEvent},
    Input,
};
use reboot_lib::Buttons;

use crate::gui::{browser::Browser, frontend::UiPage, special_thanks::SpecialThanks, AppData};

#[derive(Clone)]
pub struct MainMenu;

impl UiPage for MainMenu {
    fn ui(
        &mut self,
        ui: &mut micro_imgui_ds::micro_imgui::Ui<'_, '_, micro_imgui_ds::DSMicroGuiBackend>,
        data: &mut super::GlobalData,
    ) -> Option<Box<dyn UiPage>> {
        //ui.vertical_centered(|ui| {
        crate::focus_default(ui);
        ui.header("Welcome!");
        ui.add_space(4);
        ui.label("Astronaut made by Vikrinox, 2026");
        ui.header(" ");
        let mut res: Option<Box<dyn UiPage>> = None;
        if ui.button("Browse Files on SD").clicked() {
            if let Some(sd) = Browser::open_sd() {
                res = Some(Box::new(sd))
            }
        }
        if ui.button("Browse Files on NAND").clicked() {
            if let Some(sd) = Browser::open_nand() {
                res = Some(Box::new(sd))
            }
        }
        if ui.button("Settings").clicked() {
            res = Some(Box::new(super::settings::Settings::Main));
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

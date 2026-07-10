use alloc::boxed::Box;
use micro_imgui_ds::{micro_imgui::Backend, Input};
use reboot_lib::Buttons;

use crate::gui::{frontend::UiPage, main_menu::MainMenu};

pub struct SpecialThanks;

impl UiPage for SpecialThanks {
    fn ui(
        &mut self,
        ui: &mut micro_imgui_ds::micro_imgui::Ui<'_, '_, micro_imgui_ds::DSMicroGuiBackend>,
        _data: &mut super::GlobalData,
    ) -> Option<Box<dyn UiPage>> {
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

        if ui.input_pressed(Input(Buttons::BUTTON_B)) {
            Some(Box::new(MainMenu))
        } else {
            None
        }
    }
}

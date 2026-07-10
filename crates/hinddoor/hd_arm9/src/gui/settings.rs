use alloc::boxed::Box;
use micro_imgui_ds::{
    micro_imgui::{widgets::checkbox::Checkbox, Backend},
    Input,
};
use reboot_lib::Buttons;

use crate::gui::{frontend::UiPage, MainMenu};

pub struct Settings;

impl UiPage for Settings {
    fn ui(
        &mut self,
        ui: &mut micro_imgui_ds::micro_imgui::Ui<'_, '_, micro_imgui_ds::DSMicroGuiBackend>,
        data: &mut super::GlobalData,
    ) -> Option<alloc::boxed::Box<dyn UiPage>> {
        ui.header("Settings");
        ui.label("NOTE: These will reset on reboot, for permanent settings, use the config file.");
        ui.add_space(8);
        ui.add(Checkbox::new(
            &mut data.config.options.patch_flag,
            "DSi Menu patching",
        ));
        ui.add(Checkbox::new(
            &mut data.config.options.wifi_firmware_upload,
            "WiFi Firmware upload",
        ));
        if ui.button("go back").clicked() || ui.input_pressed(Input(Buttons::BUTTON_B)) {
            Some(Box::new(MainMenu))
        } else {
            None
        }
    }
}

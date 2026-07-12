use alloc::{boxed::Box, string::String};

use crate::gui::{
    frontend::{ClonableUiPage, UiPage},
    main_menu::MainMenu,
};

pub struct Error {
    pub error_string: String,
    pub exti: Option<Box<dyn ClonableUiPage>>,
}
impl Error {
    pub fn new(error_string: String) -> Self {
        Self {
            error_string,
            exti: None,
        }
    }
}
impl UiPage for Error {
    fn ui(
        &mut self,
        ui: &mut micro_imgui_ds::micro_imgui::Ui<'_, '_, micro_imgui_ds::DSMicroGuiBackend>,
        _data: &mut super::GlobalData,
    ) -> Option<Box<dyn UiPage>> {
        ui.header("ERROR:");
        ui.label(&self.error_string);
        if ui.button("okay").clicked() {
            Some(Box::new(MainMenu))
        } else {
            None
        }
    }
}

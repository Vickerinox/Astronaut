// SPDX-FileCopyrightText: 2026 Viktor Karlsson <viktor@koda.re>
// SPDX-License-Identifier: GPL-3.0-or-later

use alloc::{boxed::Box, string::String};

use crate::gui::{frontend::UiPage, main_menu::MainMenu};

pub struct Error {
    pub error_string: String,
}
impl Error {
    pub fn new(error_string: String) -> Self {
        Self { error_string }
    }
}
impl UiPage for Error {
    fn ui(
        &mut self,
        ui: &mut micro_imgui_ds::micro_imgui::Ui<'_, '_, micro_imgui_ds::DSMicroGuiBackend>,
        _data: &mut super::GlobalData,
    ) -> Option<Box<dyn UiPage>> {
        super::focus_default(ui);
        ui.header("ERROR:");
        ui.label(&self.error_string);
        ui.add_space(ui.clip_rect().height() - 24);
        if ui.button("oh... okay").clicked() {
            Some(Box::new(MainMenu))
        } else {
            None
        }
    }
}

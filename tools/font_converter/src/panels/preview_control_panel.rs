// SPDX-FileCopyrightText: 2026 Viktor Karlsson <viktor@koda.re>
// SPDX-License-Identifier: GPL-3.0-or-later

use eframe::egui;

use crate::{ToolData, panels::{AlwaysUi,  PanelUi}};

pub struct PreviewControlsPanel;

impl PanelUi for PreviewControlsPanel {
    fn name(&mut self) -> &str {
        "Preview Controls"
    }
}

impl AlwaysUi for PreviewControlsPanel {
    fn ui(&mut self, ui: &mut egui::Ui, data: &mut ToolData) {
        ui.horizontal(|ui| {
            ui.label("Text Color: ");
            ui.color_edit_button_srgba(&mut data.color);
            ui.label("Background color:");
            ui.color_edit_button_srgba(&mut data.background_color);
            ui.label("Use alternative palette:");
            ui.checkbox(&mut data.palette_2, "");
        });
    }
}


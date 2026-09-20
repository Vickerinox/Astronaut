// SPDX-FileCopyrightText: 2026 Viktor Karlsson <viktor@koda.re>
// SPDX-License-Identifier: GPL-3.0-or-later

use eframe::egui;

use crate::{ToolData, panels::{AlwaysUi, PanelUi}};
pub struct ColorPalettePanel;

impl PanelUi for ColorPalettePanel {
    fn name(&mut self) -> &str {
        "Color Palette"
    }
}

impl AlwaysUi for ColorPalettePanel {
    fn ui(&mut self, ui: &mut egui::Ui, data: &mut ToolData) {
        let Some(color_palette) = &mut data.palette else { ui.centered_and_justified(|ui| ui.label("No Palette available")); return; };
        ui.heading("Color Palette:");
        ui.group(|ui| {
            ui.label("Palette 1:");
            ui.horizontal_wrapped(|ui| {
                
                for color in color_palette[..4].iter_mut() {
                    if ui.color_edit_button_srgba(color).changed() {
                        data.recalculate_textures = true;
                    }
                }
            }); 
        });
        ui.group(|ui| {
            ui.label("Palette 2:");
            ui.horizontal_wrapped(|ui| {
                for color in color_palette[4..].iter_mut() {
                    if ui.color_edit_button_srgba(color).changed() {
                        data.recalculate_textures = true;
                    }
                }
            }); 
        });
    }
}
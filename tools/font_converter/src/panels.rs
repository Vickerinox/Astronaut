// SPDX-FileCopyrightText: 2026 Viktor Karlsson <viktor@koda.re>
// SPDX-License-Identifier: GPL-3.0-or-later

use eframe::egui;
use super::{ToolData, DecodedBMP};
use std::path::PathBuf;


mod preview_control_panel;
mod preview_panel;
mod color_palette_panel;
pub use preview_control_panel::*;
pub use preview_panel::*;
pub use color_palette_panel::*;

pub trait AlwaysUi: PanelUi {
    fn ui(&mut self, ui: &mut egui::Ui, data: &mut ToolData);
}
pub trait LoadedUi: PanelUi {
    fn ui(&mut self, ui: &mut egui::Ui, data: &mut ToolData, image: &DecodedBMP, bmp_path: &PathBuf);
}
pub trait PanelUi {
    fn name(&mut self) -> &str;
}
pub trait AsPanel {
    fn as_panel(self) -> PanelType;
}
pub enum PanelType {
    Always(Box<dyn AlwaysUi>),
    Loaded(Box<dyn LoadedUi>),
}
pub struct Panel {
    pub id: u64,
    pub ui: PanelType,
}







impl Panel {
    pub fn new(panel: impl AsPanel, id: u64) -> Panel {
        Panel { id, ui: panel.as_panel() }
    }
}

impl<T: AlwaysUi + 'static> AsPanel for T {
    fn as_panel(self) -> PanelType {
        PanelType::Always(Box::new(self))
    }
}
impl AsPanel for PreviewPanel {
    fn as_panel(self) -> PanelType {
        PanelType::Loaded(Box::new(self))
    }
}


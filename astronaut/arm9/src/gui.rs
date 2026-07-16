// SPDX-FileCopyrightText: 2026 Viktor Karlsson <viktor@koda.re>
// SPDX-License-Identifier: GPL-3.0-or-later

pub use micro_imgui_ds::gui::DSMicroGuiBackend;

mod backend;
pub use frontend::{AppData, GlobalData};
mod browser;
mod error;
mod frontend;
mod main_menu;
mod special_thanks;
pub use main_menu::MainMenu;
mod settings;
pub use frontend::pop_dir_entry;

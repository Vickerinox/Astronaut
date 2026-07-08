pub use micro_imgui_ds::gui::{DSMicroGuiBackend, Input};

mod backend;
pub use frontend::{AppData, MusicPlaying, GlobalData, CurrentFrontend};
mod frontend;
mod special_thanks;
mod browser;
mod main_menu;
mod error;
pub use main_menu::MainMenu;
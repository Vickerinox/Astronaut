pub use micro_imgui_ds::gui::{DSMicroGuiBackend, Input};
use micro_imgui_ds::micro_imgui::{Rect, Vec2};
use reboot_lib::{VertexListHost, VertexListType, VideoHardwareHandle};

mod backend;
pub use frontend::{AppData, CurrentUI, MusicPlaying, GlobalData, CurrentFrontend};
mod frontend;
mod special_thanks;
mod browser;
mod main_menu;
mod error;
pub use main_menu::MainMenu;
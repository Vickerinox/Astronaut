use alloc::{string::{String, ToString}, vec::Vec};
use fatfs_embedded::fatfs::FileOptions;
use micro_imgui_ds::micro_imgui::{Color, ColorSet, Style};
use reboot_lib::{Buttons, VIDEO_HARDWARE};

use crate::{get_extension, gui::pop_dir_entry, transfer_font_to_vram};

pub struct Config {
    pub style: Pretties,
    pub options: Options,
    pub autoboot: String,
    pub ini: String,
}
pub struct Pretties {
    pub music: String,
    pub top_wallpaper: String,
    pub theme_path: String,
    pub colors: micro_imgui_ds::micro_imgui::Style,
    pub folder_color: Color,
    pub bootable_color: Color,
    pub asset_color: Color,
}
pub struct Options {
    pub patch_flag: bool,
    pub wifi_firmware_upload: bool,
}
impl Options {
    pub const fn default() -> Self {
        Self {
            patch_flag: true,
            wifi_firmware_upload: true,
        }
    }
}
impl Pretties {
    pub const fn default() -> Self {
        Self {
            music: String::new(),
            top_wallpaper: String::new(),
            theme_path: String::new(),
            colors: Style::DEFAULT,
            folder_color: Color::new(200, 100, 100),
            bootable_color: Color::new(100, 200, 100),
            asset_color: Color::new(100, 100, 200),
        }
    }
}
fn read_whole_file_to_string(path: &mut String) -> Option<String> {
    String::from_utf8(read_whole_file(path)?).ok()
}
fn read_whole_file(path: &mut String) -> Option<Vec<u8>> {
    let mut file = fatfs_embedded::open(path, FileOptions::Read).ok()?;
    let size = fatfs_embedded::size(&mut file) as usize;
    let mut path_buf = alloc::vec![0u8; size];
    crate::boot::read_all(&mut path_buf, &mut file).ok()?;
    Some(path_buf)
}
impl Config {
    pub const fn default() -> Self {
        Self {
            options: Options::default(),
            autoboot: String::new(),
            ini: String::new(),
            style: Pretties::default(),
        }
    }
    pub fn load_theme(&mut self) {
        if [".ini", ".INI"].into_iter().find(|i| self.style.theme_path.ends_with(i)).is_none() {
            return;
        }
        let mut font = String::new();
        let Some(theme) = read_whole_file_to_string(&mut self.style.theme_path) else { return };
        pop_dir_entry(&mut self.style.theme_path); // go from the ini path to it's directory, base dir for free!
        ini::Ini::new(&theme, Some(&mut |segment, key, value| match (segment, key) {
            ("[assets]", "music") => handle_path(&mut self.style.theme_path, &mut self.style.music, value),
            ("[assets]", "wallpaper") => handle_path(&mut self.style.theme_path, &mut self.style.top_wallpaper, value),
            ("[assets]", "font") => handle_path(&mut self.style.theme_path, &mut font, value),
            ("[colors]", "background") => parse_color(value, &mut self.style.colors.background_color),
            ("[colors]", "text") => parse_color(value, &mut self.style.colors.text_color),
            ("[colors]", "assets") => parse_color(value, &mut self.style.asset_color),
            ("[colors]", "roms") => parse_color(value, &mut self.style.bootable_color),
            ("[colors]", "folders") => parse_color(value, &mut self.style.folder_color),
            ("[widgets]", key) => handle_gah(&mut self.style.colors.default, key, value),
            ("[widgets.active]", key) => handle_gah(&mut self.style.colors.focused, key, value),
            ("[widgets.pressed]", key) => handle_gah(&mut self.style.colors.pressed, key, value), 
            _ => (),
        }));
        let Some(font) = read_whole_file(&mut font) else { return };
        let Some(([a,b], rem)) = font.split_first_chunk() else { return };
        if a & b == 0 {
            for i in 0..rem.len() {
                unsafe { (0x2ff_1000 as *mut u8).add(i).write(rem[i]);}
            }
        }
        unsafe { transfer_font_to_vram(); }
    }
    pub fn load(held_buttons: Buttons) -> Self {
        let mut defaults = Self::default();
        let Some(str) = read_whole_file_to_string(&mut "sdmc:/_nds/vlaunch/settings.ini".to_string()) else {
            return defaults;
        };
        let current_combo = alloc::format!("h{:04x}", held_buttons.bits());
        ini::Ini::new(
            &str,
            Some(&mut |segment, key, value| match (segment, key, value) {
                ("[boot]", key, value) => {
                    if defaults.autoboot.is_empty() && key == "default" {
                        defaults.autoboot = value.to_string();
                    } else if key == &current_combo {
                        defaults.autoboot = value.to_string();
                    }
                }
                ("[options]", "wifi_firm_upload", "on") => {
                    defaults.options.wifi_firmware_upload = true;
                }
                ("[options]", "wifi_firm_upload", "off") => {
                    defaults.options.wifi_firmware_upload = false;
                }
                ("[options]", "patching", "on") => {
                    defaults.options.patch_flag = true;
                }
                ("[options]", "patching", "off") => {
                    defaults.options.patch_flag = false;
                }
                ("[style]", "wallpaper", value) => {
                    defaults.style.top_wallpaper = value.to_string();
                }
                ("[style]", "music", value) => {
                    defaults.style.music = value.to_string();
                }
                ("[style]", "theme", value) => {
                    defaults.style.theme_path = value.to_string();
                }
                _ => (),
            }),
        );
        defaults
    }
}
fn handle_gah(set: &mut ColorSet, key: &str, value: &str) {
    match key {
        "fill" => parse_color(value, &mut set.frame_fill),
        "outline" => parse_color(value, &mut set.frame_outline),
        _ => (),
    }
}
fn handle_path(base: &String, var: &mut String, value: &str) {
    if var.is_empty() {
        if value.starts_with(".") {
            *var = base.clone() + value;
        } else {
            *var = value.to_string();
        }
    }
}
pub mod ini;
fn parse_color(color: &str, var: &mut Color) {
    match color.len() {
        4 => if let Ok(color) = u32::from_str_radix(color, 16) {
            *var = Color(color as u16);
        },
        6 => if let Ok(color) = u32::from_str_radix(color, 16) {
            let [r, g, b, _] = color.to_le_bytes();
            *var = Color::new(r, g, b);
        },
        _ => (),
    }
}

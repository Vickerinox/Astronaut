use alloc::string::{String, ToString};
use fatfs_embedded::fatfs::FileOptions;
use micro_imgui_ds::micro_imgui::{Color, Style};
use reboot_lib::Buttons;

pub struct Config {
    pub style: Pretties,
    pub options: Options,
    pub autoboot: String,
    pub ini: String,
}
pub struct Pretties {
    pub music: String,
    pub top_wallpaper: String,
    pub colors: micro_imgui_ds::micro_imgui::Style,
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
            colors: Style::DEFAULT,
        }
    }
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
    pub fn load(held_buttons: Buttons) -> Self {
        let mut defaults = Self::default();
        let Ok(mut file) = 
        fatfs_embedded::open(
            &mut "sdmc:/_nds/vlaunch/settings.ini".to_string(),
            FileOptions::Read,
        ) else {return defaults};

        let size = fatfs_embedded::size(&mut file) as usize;
        let mut path_buf = alloc::vec![0u8; size];
        if crate::boot::read_all(&mut path_buf, &mut file).is_err() {
            return defaults
        }
        let Ok(str) = String::from_utf8(path_buf) else {return defaults};
        let current_combo = alloc::format!("h{:04x}", held_buttons.bits());
        let mut default_path = None;
        let mut preffered_path = None;
        ini::Ini::new(&str, Some(&mut |segment, key, value| {
            match (segment, key, value) {
                ("[boot]", key, value) => {
                    if key == "default" {
                        default_path = Some(value);
                    } else if key == &current_combo {
                        preffered_path = Some(value);
                    }
                },
                ("[options]", "wifi_firm_upload", "on") => {defaults.options.wifi_firmware_upload = true;},
                ("[options]", "wifi_firm_upload", "off") => {defaults.options.wifi_firmware_upload = false;},
                ("[options]", "patching", "on") => {defaults.options.patch_flag = true;},
                ("[options]", "patching", "off") => {defaults.options.patch_flag = false;},
                ("[style]", "wallpaper", value) => {defaults.style.top_wallpaper = value.to_string();}
                ("[style]", "music", value) => {defaults.style.music = value.to_string();}
                _ => ()
            }
        }));
        if let Some(boot_path) = preffered_path.or(default_path) {
            defaults.autoboot = boot_path.to_string();
        }    
        defaults
        
    }
}

pub mod ini;
fn parse_color(color: &str) -> Option<Color> {
    match color.len() {
        4 => u32::from_str_radix(color, 16).ok().map(|i| Color(i as u16)),
        6 => {
            u32::from_str_radix(color, 16).ok().map(|i| {
                let [r,g,b,_] = i.to_le_bytes();
                Color::new(r, g, b)
            })
        }
        len => panic!("{len}"),
    }
}
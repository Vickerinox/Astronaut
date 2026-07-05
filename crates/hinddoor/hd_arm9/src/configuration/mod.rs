use alloc::string::{String, ToString};
use fatfs_embedded::fatfs::FileOptions;
use micro_imgui_ds::read_controller;
use reboot_lib::Buttons;

pub struct Config {
    pub style: Style,
    pub options: Options,
    pub autoboot: String,
    pub ini: String,
}
pub struct Style {
    pub music: String,
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
impl Style {
    pub const fn default() -> Self {
        Self {
            music: String::new(),
        }
    }
}
impl Config {
    pub const fn default() -> Self {
        Self {
            options: Options::default(),
            autoboot: String::new(),
            ini: String::new(),
            style: Style::default(),
        }
    }
    pub fn load(held_buttons: Buttons) -> Self {
        fatfs_embedded::open(
            &mut "sdmc:/_nds/vlaunch/settings.ini".to_string(),
            FileOptions::Read,
        )
        .ok()
        .and_then(|mut file| {
            let size = fatfs_embedded::size(&mut file) as usize;
            let mut path_buf = alloc::vec![0u8; size];
            crate::boot::read_all(&mut path_buf, &mut file).ok()?;
            let str = String::from_utf8(path_buf).ok()?;
            let ini = ini::Ini::new(&str);

            let current_combo = alloc::format!("h{:04x}", held_buttons.bits());

            let autoboot = ini
                .get("[boot]")
                .and_then(|i| i.get(&current_combo).or(i.get("default")))
                .unwrap_or("")
                .to_string();
            let (options, style) = if let Some(i) = ini.get("[options]") {
                let options = Options {
                    patch_flag: i.get("patching").map(|i| i == "on").unwrap_or(true),
                    wifi_firmware_upload: i.get("wifi_firm_upload").map(|i| i == "on").unwrap_or(true),
                };
                let style = Style {
                    music: i.get("music").unwrap_or("").to_string(),
                };
                (options, style)
            } else {
                (Options::default(), Style::default())
            };
            Some(Config {
                style,
                options,
                autoboot,
                ini: str,
            })
        })
        .unwrap_or(Config::default())
    }
}

pub mod ini;

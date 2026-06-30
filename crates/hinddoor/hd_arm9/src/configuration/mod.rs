use alloc::string::{String, ToString};
use fatfs_embedded::fatfs::FileOptions;
use micro_imgui_ds::read_controller;
use reboot_lib::Buttons;

pub struct Config {
    pub patch_flag: bool,
    pub autoboot: String,
    pub music: String,
    pub ini: String,
}
impl Config {
    pub const fn default() -> Self {
        Self {
            patch_flag: true,
            autoboot: String::new(),
            ini: String::new(),
            music: String::new(),
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
            
            let current_combo = alloc::format!("{:04x}h",held_buttons.bits());

            let autoboot = ini
                .get("[boot]")
                .and_then(|i| i.get(&current_combo).or(i.get("default")))
                .unwrap_or("")
                .to_string();
            let (patch_flag, music) = if let Some(i) = ini.get("[options]") {
                (i.get("patching").map(|i| i == "on"), i.get("music"))
            } else {
                (None, None)
            };
            let music = music.unwrap_or("").to_string();
            let patch_flag = patch_flag.unwrap_or(true);
            Some(Config {
                patch_flag,
                autoboot,
                music,
                ini: str,
            })
        })
        .unwrap_or(Config::default())
    }
}

pub mod ini;

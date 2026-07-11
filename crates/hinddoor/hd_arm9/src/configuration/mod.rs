use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use fatfs_embedded::fatfs::FileOptions;
use micro_imgui_ds::micro_imgui::{Color, ColorSet, Style};
use reboot_lib::{Buttons, VIDEO_HARDWARE, VideoHardwareHandle, music_modules::mods::MODAsyncLoader};

use crate::{FileType, get_extension, gui::{GlobalData, MusicPlaying, StreamingWav, pop_dir_entry}, transfer_font_to_vram};

pub struct Config {
    pub patch_flag: bool,
    pub wifi_firmware_upload: bool,
    pub autoboot: String,
    pub ini: String,
    pub music: String,
    pub top_wallpaper: String,
    pub theme: Theme,
    pub theme_path: String,
}
pub struct Theme {
    pub colors: micro_imgui_ds::micro_imgui::Style,
    pub folder_color: Color,
    pub bootable_color: Color,
    pub asset_color: Color,

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
            patch_flag: true,
            wifi_firmware_upload: true,
            autoboot: String::new(),
            ini: String::new(),
            music: String::new(),
            top_wallpaper: String::new(),
            theme_path: String::new(),
            theme: Theme {
                colors: Style::DEFAULT,
                folder_color: Color::new(200, 100, 100),
                bootable_color: Color::new(100, 200, 100),
                asset_color: Color::new(100, 100, 200),
            },
        }
    }
    
    pub fn load(held_buttons: Buttons) -> Self {
        let mut defaults = Self::default();
        let Some(str) =
            read_whole_file_to_string(&mut "sdmc:/_nds/vlaunch/settings.ini".to_string())
        else {
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
                    defaults.wifi_firmware_upload = true;
                }
                ("[options]", "wifi_firm_upload", "off") => {
                    defaults.wifi_firmware_upload = false;
                }
                ("[options]", "patching", "on") => {
                    defaults.patch_flag = true;
                }
                ("[options]", "patching", "off") => {
                    defaults.patch_flag = false;
                }
                ("[style]", "wallpaper", value) => {
                    defaults.top_wallpaper = value.to_string();
                }
                ("[style]", "music", value) => {
                    defaults.music = value.to_string();
                }
                ("[style]", "theme", value) => {
                    defaults.theme_path = value.to_string();
                }
                _ => (),
            }),
        );
        defaults
    }
}
impl GlobalData {
    unsafe fn load_theme_inner(theme_path: &mut String, music_path: &mut String, wallpaper_path: &mut String, theme: &mut Theme) {

        if [".ini", ".INI"]
            .into_iter()
            .find(|i| theme_path.ends_with(i))
            .is_none()
        {
            return;
        }
        let Some(theme_string) = read_whole_file_to_string(theme_path) else {
            return;
        };
        let mut font = String::new();
        let mut base_dir = theme_path.clone();
        pop_dir_entry(&mut base_dir); // go from the ini path to it's directory, base dir for free!
        ini::Ini::new(
            &theme_string,
            Some(&mut |segment, key, value| match (segment, key) {
                ("[assets]", "music") => {
                    handle_path(&base_dir, music_path, value)
                }
                ("[assets]", "wallpaper") => handle_path(
                    &base_dir,
                    wallpaper_path,
                    value,
                ),
                ("[assets]", "font") => handle_path(&base_dir, &mut font, value),
                ("[colors]", "background") => {
                    parse_color(value, &mut theme.colors.background_color)
                }
                ("[colors]", "text") => parse_color(value, &mut theme.colors.text_color),
                ("[colors]", "assets") => parse_color(value, &mut theme.asset_color),
                ("[colors]", "roms") => parse_color(value, &mut theme.bootable_color),
                ("[colors]", "folders") => parse_color(value, &mut theme.folder_color),
                ("[widgets]", key) => handlecolorset(&mut theme.colors.default, key, value),
                ("[widgets.active]", key) => handlecolorset(&mut theme.colors.focused, key, value),
                ("[widgets.pressed]", key) => {
                    handlecolorset(&mut theme.colors.pressed, key, value)
                }
                _ => (),
            }),
        );
        let Some(font) = read_whole_file(&mut font) else {
            return;
        };
        let Some(([a, b], rem)) = font.split_first_chunk() else {
            return;
        };
        if a & b == 0 {
            for i in 0..rem.len() {
                unsafe {
                    (0x2ff_1000 as *mut u8).add(i).write(rem[i]);
                }
            }
        } 
        
    }

    fn play_startup_music(path: &mut String) -> MusicPlaying {
        let Ok(file) =
            fatfs_embedded::open(path, FileOptions::Read) else { return MusicPlaying::None };
        let Some(extension) = get_extension(path.as_bytes()) else { return MusicPlaying::None };
        let a = extension.to_ascii_uppercase();
        match crate::filetype(&a) {
            FileType::Mod => {
                MusicPlaying::Mod(MODAsyncLoader::new(file))
            }
            FileType::Wav => {
                if let Some(mut stream) = StreamingWav::new(file) {
                     unsafe {
                        stream.play();
                    }
                    MusicPlaying::Wav(stream)
                } else { MusicPlaying::None }
            }
            _ => MusicPlaying::None
        }
    }
    fn load_wallpaper(path: &mut String) -> Option<crate::bmp::DecodedBMP> {
        let file = fatfs_embedded::open(
            path,
            FileOptions::Read,
        )
        .ok()?;
        crate::bmp::DecodedBMP::from_reader(file)
    }
    pub unsafe fn load_theme(&mut self) -> (Style, VideoHardwareHandle) {
        let mut music = self.config.music.clone();
        let mut wallpaper = self.config.top_wallpaper.clone();
        crate::load_default_font();
        Self::load_theme_inner(&mut self.config.theme_path, &mut music, &mut wallpaper, &mut self.config.theme);
        let video_context = crate::init_graphics();
        if let Some(wallpaper) = Self::load_wallpaper(&mut wallpaper) {
            crate::show_wallpaper(wallpaper);
        }
        self.loading_mod_file = Self::play_startup_music(&mut music);
        (self.config.theme.colors.clone(), video_context)
    }
}
fn handlecolorset(set: &mut ColorSet, key: &str, value: &str) {
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
        4 => {
            if let Ok(color) = u32::from_str_radix(color, 16) {
                *var = Color(color as u16);
            }
        }
        6 => {
            if let Ok(color) = u32::from_str_radix(color, 16) {
                let [b, r, g, _] = color.to_le_bytes();
                *var = Color::new(r, g, b);
            }
        }
        _ => (),
    }
}

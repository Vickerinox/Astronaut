use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};
use fatfs_embedded::fatfs::FileOptions;
use micro_imgui_ds::micro_imgui::{Color, ColorSet, Style};
use reboot_lib::{
    music_modules::mods::MODAsyncLoader, Buttons, DisplayControl, VRAMCtrl, VideoHardwareHandle,
    VIDEO_HARDWARE,
};

use crate::{
    get_extension,
    gui::{pop_dir_entry, GlobalData, MusicPlaying, StreamingWav},
    transfer_font_to_vram, FileType,
};

pub struct BootCombo {
    pub buttons: Buttons,
    pub path: String,
}

pub struct BootCombos {
    pub default: String,
    pub additionals: Vec<BootCombo>,
}
impl BootCombos {
    pub const fn default() -> Self {
        Self {
            default: String::new(),
            additionals: Vec::new(),
        }
    }
    pub fn add(&mut self, combo: BootCombo) {
        if let Some(existing_entry) = self
            .additionals
            .iter_mut()
            .filter(|i| i.buttons == combo.buttons)
            .next()
        {
            existing_entry.path = combo.path;
        } else {
            self.additionals.push(combo);
        }
    }
    pub fn set_default(&mut self, path: String) {
        self.default = path
    }
}
pub struct Config {
    pub patch_flag: bool,
    pub wifi_firmware_upload: bool,
    pub autoboot: String,
    pub ini: String,
    pub music: String,
    pub top_wallpaper: String,
    pub theme: Theme,
    pub theme_path: String,
    pub boot_combos: BootCombos,
}

impl Config {
    pub fn into_ini(&self) -> String {
        let mut ini = String::new();
        ini.push_str("[options]\n");
        ini.push_str("wifi_firm_upload");
        if self.wifi_firmware_upload {
            ini.push_str("=on\n");
        } else {
            ini.push_str("=off\n");
        }
        ini.push_str("patching");
        if self.patch_flag {
            ini.push_str("=on\n");
        } else {
            ini.push_str("=off\n");
        }

        ini.push_str("\n[style]\n");
        if !self.theme_path.is_empty() {
            ini.push_str(&format!("theme={}\n", &self.theme_path));
        }
        if !self.music.is_empty() {
            ini.push_str(&format!("music={}\n", &self.music));
        }
        if !self.top_wallpaper.is_empty() {
            ini.push_str(&format!("wallpaper={}\n", &self.top_wallpaper));
        }

        ini.push_str("\n[boot]\n");

        if !self.boot_combos.default.is_empty() {
            ini.push_str(&format!("default={}\n", &self.boot_combos.default));
        }
        for combo in &self.boot_combos.additionals {
            ini.push_str(&format!("h{:04x}={}\n", combo.buttons.bits(), &combo.path));
        }
        ini
    }
}
pub struct Theme {
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
            boot_combos: BootCombos::default(),
            theme: Theme {
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
        ini::Ini::new(
            &str,
            Some(&mut |segment, key, value| match (segment, key, value) {
                ("[boot]", key, value) => {
                    if defaults.autoboot.is_empty() && key == "default" {
                        defaults.autoboot = value.to_string();
                        defaults.boot_combos.default = value.to_string();
                    } else {
                        let Some((code, remainder)) = key.split_at_checked(1) else {
                            return;
                        };
                        if code != "h" {
                            return;
                        };
                        let Ok(combo) = u16::from_str_radix(remainder, 16) else {
                            return;
                        };
                        let combo = Buttons::from_bits_truncate(combo);

                        defaults.boot_combos.add(BootCombo {
                            buttons: combo,
                            path: value.to_string(),
                        });
                        if held_buttons == combo {
                            defaults.autoboot = value.to_string();
                        }
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
fn read_ini(path: &mut String) -> Option<String> {
    [".ini", ".INI"].into_iter().find(|i| path.ends_with(i))?;
    read_whole_file_to_string(path)
}
impl GlobalData {
    unsafe fn load_theme_inner(
        theme_path: &mut String,
        music_path: &mut String,
        wallpaper_path: &mut String,
        background_path: &mut String,
        theme: &mut Theme,
    ) -> Style {
        let mut style = Style::DEFAULT;
        let Some(theme_string) = read_ini(theme_path) else {
            crate::load_default_font();
            return style;
        };
        let mut font = String::new();
        let mut base_dir = theme_path.clone();
        pop_dir_entry(&mut base_dir);
        ini::Ini::new(
            &theme_string,
            Some(&mut |segment, key, value| match (segment, key) {
                ("[assets]", "music") => handle_path(&base_dir, music_path, value),
                ("[assets]", "wallpaper") => handle_path(&base_dir, wallpaper_path, value),
                ("[assets]", "background") => handle_path(&base_dir, background_path, value),
                ("[assets]", "font") => handle_path(&base_dir, &mut font, value),
                ("[colors]", "background") => parse_color(value, &mut style.background_color),
                ("[colors]", "text") => parse_color(value, &mut style.text_color),
                ("[colors]", "assets") => parse_color(value, &mut theme.asset_color),
                ("[colors]", "roms") => parse_color(value, &mut theme.bootable_color),
                ("[colors]", "folders") => parse_color(value, &mut theme.folder_color),
                ("[widgets]", key) => handlecolorset(&mut style.default, key, value),
                ("[widgets.active]", key) => handlecolorset(&mut style.focused, key, value),
                ("[widgets.pressed]", key) => handlecolorset(&mut style.pressed, key, value),
                _ => (),
            }),
        );
        let Some(font) = read_whole_file(&mut font) else {
            crate::load_default_font();
            return style;
        };
        let Some(([a, b], rem)) = font.split_first_chunk() else {
            crate::load_default_font();
            return style;
        };
        if a & b == 0 {
            for i in 0..rem.len() {
                unsafe {
                    (0x2ff_1000 as *mut u8).add(i).write(rem[i]);
                }
            }
        }
        style
    }

    fn play_startup_music(path: &mut String) -> MusicPlaying {
        let Ok(file) = fatfs_embedded::open(path, FileOptions::Read) else {
            return MusicPlaying::None;
        };
        let Some(extension) = get_extension(path.as_bytes()) else {
            return MusicPlaying::None;
        };
        let a = extension.to_ascii_uppercase();
        match crate::filetype(&a) {
            FileType::Mod => MusicPlaying::Mod(MODAsyncLoader::new(file)),
            FileType::Wav => {
                if let Some(mut stream) = StreamingWav::new(file) {
                    unsafe {
                        stream.play();
                    }
                    MusicPlaying::Wav(stream)
                } else {
                    MusicPlaying::None
                }
            }
            _ => MusicPlaying::None,
        }
    }
    fn load_wallpaper(path: &mut String) -> Option<crate::bmp::DecodedBMP> {
        let file = fatfs_embedded::open(path, FileOptions::Read).ok()?;
        crate::bmp::DecodedBMP::from_reader(file)
    }
    pub unsafe fn load_theme(&mut self) -> (Style, VideoHardwareHandle) {
        let mut music = self.config.music.clone();
        let mut wallpaper = self.config.top_wallpaper.clone();
        let mut background = String::new();

        let colors = Self::load_theme_inner(
            &mut self.config.theme_path,
            &mut music,
            &mut wallpaper,
            &mut background,
            &mut self.config.theme,
        );
        if let Some(wallpaper) = Self::load_wallpaper(&mut wallpaper) {
            VIDEO_HARDWARE
                .vram_control_bank_c
                .write(VRAMCtrl::ENABLE | VRAMCtrl::LCD_MAPPED);

            crate::show_wallpaper(wallpaper, 0x06840000 as *mut u16);

            VIDEO_HARDWARE
                .vram_control_bank_c
                .write(VRAMCtrl::ENABLE | VRAMCtrl::MST_4);
            VIDEO_HARDWARE
                .disp_b_control
                .write(DisplayControl::BG_MODE_5 | DisplayControl::ENABLE_BG_3);
        } else {
            VIDEO_HARDWARE
                .disp_b_control
                .write(DisplayControl::BG_MODE_5);
        }
        VIDEO_HARDWARE
            .vram_control_bank_a
            .write(VRAMCtrl::ENABLE | VRAMCtrl::LCD_MAPPED);

        if let Some(background) = Self::load_wallpaper(&mut background) {
            crate::show_wallpaper(background, 0x06800000 as *mut u16);
        } else {
            for i in 0..(256 * 192) {
                (0x06800000 as *mut u16)
                    .add(i)
                    .write(colors.background_color.0 | 0x8000);
            }
        }
        let video_context = crate::init_graphics();

        self.loading_mod_file = Self::play_startup_music(&mut music);
        (colors, video_context)
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
                let [b, g, r, _] = color.to_le_bytes();
                *var = Color::new(r, g, b);
            }
        }
        7 | 8 => {
            if let Ok(color) = u32::from_str_radix(color, 16) {
                let [b, g, r, a] = color.to_le_bytes();

                if a == 0 {
                    *var = Color::new_transparent(r, g, b)
                } else {
                    *var = Color::new(r, g, b);
                }
            }
        }
        _ => (),
    }
}

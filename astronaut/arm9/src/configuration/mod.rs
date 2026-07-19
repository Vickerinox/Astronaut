// SPDX-FileCopyrightText: 2026 Viktor Karlsson <viktor@koda.re>
// SPDX-License-Identifier: GPL-3.0-or-later

use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};

use fatfs_embedded::fatfs::FileOptions;
use micro_imgui_ds::micro_imgui::{Color, ColorSet, Style};
use reboot_lib::fatfs_embedded;
use reboot_lib::{
    music_modules::mods::MODAsyncLoader, Buttons, DisplayControl, VRAMCtrl, VideoHardwareHandle,
    VIDEO_HARDWARE,
};

use crate::{
    get_extension,
    gui::{pop_dir_entry, GlobalData},
    music::{MusicPlaying, StreamingWav},
    FileType,
};

pub struct BootCombo {
    pub buttons: Buttons,
    pub path: String,
}
impl BootCombo {
    pub fn new(buttons: Buttons, path: String) -> Self {
        Self { buttons, path }
    }
}

pub struct BootCombos {
    pub default: String,
    pub additionals: Vec<BootCombo>,
    partial_init: Option<Buttons>,
}
impl BootCombos {
    pub const fn default() -> Self {
        Self {
            default: String::new(),
            additionals: Vec::new(),
            partial_init: None,
        }
    }
    pub fn start(&mut self, buttons: Buttons) {
        self.partial_init = Some(buttons);
    }
    pub fn finish(&mut self, string: String) {
        if let Some(buttons) = self.partial_init.take() {
            if buttons.is_empty() {
                self.default = string;
            } else {
                self.add(BootCombo::new(buttons, string));
            }
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
    pub music: String,
    pub top_wallpaper: String,
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
impl Theme {
    pub const DEFAULT: Self = Theme { 
        folder_color: Color::new(200, 100, 100),
        bootable_color: Color::new(100, 200, 100), 
        asset_color: Color::new(100, 100, 200),
    };
    #[no_mangle] 
    #[link_section = ".text_aux"]
    pub fn load(&mut self, theme_path: &mut String) -> (Assets, Style) {
        let mut assets = Assets {
            music: String::new(),
            wallpaper: String::new(),
            background: String::new(),
            font: String::new(),
        };
        let mut style = Style::DEFAULT;
        let Some(theme_string) = read_ini(theme_path) else {
            return (assets, style);
        };
        let mut base_dir = theme_path.clone();
        pop_dir_entry(&mut base_dir);
        ini::Ini::new(
            &theme_string,
            Some(&mut |segment, key, value| match (segment, key) {
                ("[assets]", "music") => handle_path(&base_dir, &mut assets.music, value),
                ("[assets]", "wallpaper") => handle_path(&base_dir, &mut assets.wallpaper, value),
                ("[assets]", "background") => handle_path(&base_dir, &mut assets.background, value),
                ("[assets]", "font") => handle_path(&base_dir, &mut assets.font, value),
                ("[colors]", "background") => parse_color(value, &mut style.background_color),
                ("[colors]", "text") => parse_color(value, &mut style.text_color),
                ("[colors]", "assets") => parse_color(value, &mut self.asset_color),
                ("[colors]", "roms") => parse_color(value, &mut self.bootable_color),
                ("[colors]", "folders") => parse_color(value, &mut self.folder_color),
                ("[widgets]", key) => handlecolorset(&mut style.default, key, value),
                ("[widgets.active]", key) => handlecolorset(&mut style.focused, key, value),
                ("[widgets.pressed]", key) => handlecolorset(&mut style.pressed, key, value),
                _ => (),
            }),
        );
        (assets, style)
    }
}
pub struct Assets {
    music: String,
    wallpaper: String,
    background: String,
    font: String,
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
            music: String::new(),
            top_wallpaper: String::new(),
            theme_path: String::new(),
            boot_combos: BootCombos::default(),
        }
    }

    pub fn load(&mut self, held_buttons: Buttons) {
        let Some(str) =
            read_whole_file_to_string(&mut "sdmc:/_nds/astronaut/settings.ini".to_string())
                .or_else(|| read_whole_file_to_string(&mut "nand:/astronaut.ini".to_string()))
        else {
            return;
        };
        ini::Ini::new(
            &str,
            Some(&mut |segment, key, value| match (segment, key, value) {
                ("[boot]", key, value) => {
                    if self.autoboot.is_empty() && key == "default" {
                        self.autoboot = value.to_string();
                        self.boot_combos.default = value.to_string();
                    } else {
                        let combo_parse = key
                            .split_at_checked(1)
                            .and_then(|(i, j)| (i == "h").then_some(j))
                            .and_then(|j| u32::from_str_radix(j, 16).ok());
                        let Some(combo) = combo_parse else {
                            return;
                        };
                        let combo = Buttons::from_bits_truncate(combo as u16);
                        self.boot_combos
                            .add(BootCombo::new(combo, value.to_string()));
                        if held_buttons == combo {
                            self.autoboot = value.to_string();
                        }
                    }
                }
                ("[options]", "wifi_firm_upload", "on") => {
                    self.wifi_firmware_upload = true;
                }
                ("[options]", "wifi_firm_upload", "off") => {
                    self.wifi_firmware_upload = false;
                }
                ("[options]", "patching", "on") => {
                    self.patch_flag = true;
                }
                ("[options]", "patching", "off") => {
                    self.patch_flag = false;
                }
                ("[style]", "wallpaper", value) => {
                    self.top_wallpaper = value.to_string();
                }
                ("[style]", "music", value) => {
                    self.music = value.to_string();
                }
                ("[style]", "theme", value) => {
                    self.theme_path = value.to_string();
                }
                _ => (),
            }),
        );
    }
}
fn read_ini(path: &mut String) -> Option<String> {
    [".ini", ".INI"].into_iter().find(|i| path.ends_with(i))?;
    read_whole_file_to_string(path)
}
impl GlobalData {
    fn play_startup_music(path: &mut String) -> MusicPlaying {
        let Ok(file) = fatfs_embedded::open(path, FileOptions::Read) else {
            return MusicPlaying::None;
        };
        let Some(extension) = get_extension(path) else {
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
    #[no_mangle]
    #[link_section = ".text_aux"]
    pub unsafe fn load_theme(&mut self, assets: Assets) -> VideoHardwareHandle {
        let Assets {
            mut music,
            mut wallpaper,
            mut background,
            mut font,
        } = assets;
        if let Some(font) = load_font(&mut font) {
            if load_font_real(font).is_none() {
                crate::load_default_font();
            }
        } else {
            crate::load_default_font();
        }
        let wp = if self.config.top_wallpaper.is_empty() {
            &mut wallpaper
        } else {
            &mut self.config.top_wallpaper
        };
        if let Some(wallpaper) = Self::load_wallpaper(wp) {
            VIDEO_HARDWARE
                .vram_control_bank_c
                .write(VRAMCtrl::ENABLE | VRAMCtrl::LCD_MAPPED);

            crate::gui::show_wallpaper(wallpaper, 0x06840000 as *mut u16);

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
            crate::gui::show_wallpaper(background, 0x06800000 as *mut u16);
        } else {
            for i in 0..(256 * 192) {
                (0x06800000 as *mut u16).add(i).write(0x8000);
            }
        }

        let video_context = crate::init_graphics();
        let m = if self.config.music.is_empty() {
            &mut music
        } else {
            &mut self.config.music
        };
        self.loading_mod_file = Self::play_startup_music(m);
        video_context
    }
}
fn load_font(path: &mut String) -> Option<Vec<u8>> {
    read_whole_file(path)
}
unsafe fn load_font_real(font: Vec<u8>) -> Option<()> {
    if !(0x806..0x814).contains(&font.len()) {
        return None;
    }
    let mut iter = font.chunks_exact(2);
    if iter.next()? != &[0, 0] {
        return None;
    }
    for i in 0..0x408 {
        (0x2ff1000 as *mut u16).add(i).write(
            iter.next()
                .map(|i| Some(u16::from_le_bytes(i.first_chunk().cloned()?)))
                .flatten()
                .unwrap_or(0),
        );
    }

    Some(())
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
fn parse_color(color_str: &str, var: &mut Color) {
    let Ok(color) = u32::from_str_radix(color_str, 16) else {
        return;
    };
    let [b, g, r, a] = color.to_le_bytes();
    *var = match color_str.len() {
        4 => Color(color as u16),
        6 => Color::new(r, g, b),
        7 | 8 => {
            if a == 0 {
                Color::new_transparent(r, g, b)
            } else {
                Color::new(r, g, b)
            }
        }
        _ => return,
    };
}

use crate::{
    boot,
    gui::{self, backend::Inputs, Input},
    is_bootable, is_music_module, populate_fs_vec, send_mod_file, stop_mod_file, COLOR_BOOTABLE,
    COLOR_MUSIC,
};
use alloc::{
    boxed::Box,
    format,
    string::{String, ToString},
    vec::{self, Vec},
};
use fatfs_embedded::fatfs::{File, FileInfo, FileOptions, FS_SD};
use micro_imgui::{widgets::button::Button, Backend, Color, Sizing, Vec2};
use reboot_lib::{
    music_modules::mods::{MODAsyncLoader, MODHeader},
    Buttons,
};
enum CurrentUI {
    None,
    Error {
        error_string: String,
    },
    Browsing {
        immediate_files: Vec<(String, String, bool, Color)>,
        file_path: String,
        is_nand: bool,
        offset: usize,
    },
    LoadingApp {
        file: fatfs_embedded::fatfs::File,
        file_path: String,
    },
    LoadingMusic {
        file: fatfs_embedded::fatfs::File,
        file_path: String,
    },
}

pub struct AppData {
    current_dir: CurrentUI,
    loading_mod_file: Option<MODAsyncLoader>,
}
impl AppData {
    pub fn new() -> Self {
        Self {
            current_dir: CurrentUI::None,
            loading_mod_file: None,
        }
    }
    pub fn open_sd() -> Option<CurrentUI> {
        let mut file_path = String::from("sd:/");
        fatfs_embedded::opendir(&mut file_path).ok().map(|mut i| {
            let immediate_files = populate_fs_vec(&mut i);
            CurrentUI::Browsing {
                immediate_files,
                file_path,
                is_nand: false,
                offset: 0,
            }
        })
    }
    pub fn open_nand() -> Option<CurrentUI> {
        let mut file_path = String::from("nand:/");
        fatfs_embedded::opendir(&mut file_path).ok().map(|mut i| {
            let immediate_files = populate_fs_vec(&mut i);
            CurrentUI::Browsing {
                immediate_files,
                file_path,
                is_nand: false,
                offset: 0,
            }
        })
    }
    pub fn open_default_fs() -> CurrentUI {
        Self::open_sd()
            .or_else(|| Self::open_nand())
            .unwrap_or(CurrentUI::Error {
                error_string: String::from("No Filesystem could be mounted."),
            })
    } 
    pub unsafe fn autoboot(&self) {
        match fatfs_embedded::open(
                &mut "sd:/_nds/vlaunch/autoboot.bin".to_string(),
                FileOptions::Read,
            ) {
                Ok(mut file) => {
                    let size = fatfs_embedded::size(&mut file) as usize;
                    let mut path_buf: Vec<u8> = alloc::vec![0; size];
                    if fatfs_embedded::read(&mut file, &mut path_buf).is_err() {
                        return;
                    }
                    let Ok(mut str) = String::from_utf8(path_buf) else {
                        return;
                    };
                    let Ok(mut file) = fatfs_embedded::open(&mut str, FileOptions::Read) else {
                        return;
                    };
                    crate::boot::boot_app(&mut file, &str);
                }
                Err(_abort) => {}
            }
    }
    pub fn play_startup_music(&mut self) {
        match fatfs_embedded::open(
            &mut "sd:/_nds/vlaunch/music.bin".to_string(),
            FileOptions::Read,
        ) {
            Ok(mut file) => {
                let size = fatfs_embedded::size(&mut file) as usize;
                let mut path_buf: Vec<u8> = alloc::vec![0; size];
                if fatfs_embedded::read(&mut file, &mut path_buf).is_err() {
                    return;
                }
                let Ok(mut str) = String::from_utf8(path_buf) else {
                    return;
                };
                let Ok(file) = fatfs_embedded::open(&mut str, FileOptions::Read) else {
                    return;
                };
                stop_mod_file();
                self.loading_mod_file = Some(MODAsyncLoader::new(file));
            }
            Err(_abort) => {}
        }
    }
    pub fn update(&mut self, f: &mut micro_imgui::Frame<'_, super::DSMicroGuiBackend>) {
        let mouse = f.last_known_pointer_location();
        f.central_panel(|ui| {
            unsafe {
                ui.label(&format!("SD stat: {:?} NAND stat: {:?}", crate::SD_ERROR, crate::EMMC_ERROR));
            }
            if let Some(loading_mod) = self.loading_mod_file.take() {
                let (progress, max) = loading_mod.progress();
                let progress_bar = progress * 30 / max;
                let bar = (0..30)
                    .map(|i| if i < progress_bar { '=' } else { '.' })
                    .collect::<String>();
                ui.label(&format!("Loading [{bar}]"));
                ui.request_repaint();
                match loading_mod.process() {
                    Ok(Some(ret)) => {
                        send_mod_file(ret);
                    }
                    Ok(None) => (),
                    Err(cont) => self.loading_mod_file = Some(cont),
                }
            } else {
                ui.label(" ");
            }

            let new_state_fn: Option<Box<dyn FnOnce(CurrentUI) -> CurrentUI>> =
                match &mut self.current_dir {
                    CurrentUI::Error { error_string } => {
                        ui.header("ERROR:");
                        ui.label(error_string);
                        if ui.button("okay").clicked() {
                            Some(Box::new(|_| CurrentUI::None))
                        } else {
                            None
                        }
                    },
                    CurrentUI::None => {
                        ui.header("Welcome!");
                        ui.label("Made by Vikrinox, 2026");
                        ui.header(" ");
                        let mut res: Option<Box<dyn FnOnce(CurrentUI) -> CurrentUI>> = None;
                        if ui.button("Browse Files on SD").clicked() {
                            if let Some(sd) = Self::open_sd() {
                                res = Some(Box::new(move |_| sd))
                            }
                        }
                        if ui.button("Browse Files on NAND").clicked() {
                            if let Some(sd) = Self::open_nand() {
                                res = Some(Box::new(move |_| sd))
                            } 
                        } 
                        if ui.button("Test crashing").clicked() {
                            panic!("Testing crashes");
                        }
                        res
                    }
                    CurrentUI::LoadingApp { file, file_path } => {
                        ui.request_repaint();
                        let error = unsafe { boot::boot_app(file, &file_path) };
                        let error_string = alloc::format!("Failed to boot file: {error:?}");
                        Some(Box::new(|_| CurrentUI::Error { error_string }))
                    }
                    CurrentUI::LoadingMusic { file, file_path } => {
                        if ui.button("Play song").clicked() {
                            if fatfs_embedded::seek(file, 0) == Ok(()) {
                                Some(Box::new(|s| {
                                    if let CurrentUI::LoadingMusic { file, file_path } = s {
                                        self.loading_mod_file = Some(MODAsyncLoader::new(file));
                                    }
                                    CurrentUI::None
                                }))
                            } else {
                                Some(Box::new(|_| CurrentUI::Error {
                                    error_string: "Error loading MOD".into(),
                                }))
                            }
                        } else if ui.button("go back").clicked() {
                            Some(Box::new(|_| CurrentUI::None))
                        } else if ui.button("set default").clicked() {
                            let mut file = match fatfs_embedded::open(
                                &mut "sd:/_nds/vLaunch/music.bin".to_string(),
                                FileOptions::Write | FileOptions::CreateAlways,
                            ) {
                                Ok(file) => file,
                                Err(what) => panic!("{:?}", what),
                            };
                            if file_path.len() < 1000 {
                                let bytes = file_path.as_bytes();
                                match fatfs_embedded::write(&mut file, bytes) {
                                    Ok(len) => assert_eq!(len as usize, bytes.len()),
                                    _ => panic!(),
                                };
                                fatfs_embedded::truncate(&mut file).unwrap();
                                unsafe { FS_SD.sync(&mut file).unwrap() };
                            }

                            pop_dir_entry(file_path);
                            None
                        } else {
                            None
                        }
                    }
                    CurrentUI::Browsing {
                        immediate_files,
                        file_path: current_path,
                        is_nand,
                        offset,
                    } => {
                        if ui.input_pressed(Input(Buttons::DIRECTION_UP)) {
                            *offset = offset.saturating_sub(1);
                        }

                        if ui.input_pressed(Input(Buttons::DIRECTION_DOWN)) {
                            *offset = offset.saturating_add(1);
                        }

                        let shown_items = immediate_files.get(*offset..).unwrap_or(&[]);
                        let shown_items = if let Some(clamped) = shown_items.get(..11) {
                            clamped
                        } else {
                            shown_items
                        };

                        let mut new_state: Option<
                            alloc::boxed::Box<dyn FnOnce(CurrentUI) -> CurrentUI>,
                        > = None;
                        let mut new_folder = None;
                        ui.label(current_path);
                        for item in shown_items.iter() {
                            if ui
                                .add(Button::new(
                                    &item.0,
                                    Sizing::Padded(Vec2::new(248, 8)),
                                    item.3,
                                ))
                                .clicked()
                            {
                                if item.2 {
                                    current_path.push_str(&item.1);
                                    current_path.push('/');
                                    if let Ok(f) = fatfs_embedded::opendir(current_path) {
                                        new_folder = Some(f);
                                    }
                                } else {
                                    if item.3 == COLOR_BOOTABLE {
                                        current_path.push_str(&item.1);
                                        match fatfs_embedded::open(current_path, FileOptions::Read)
                                        {
                                            Ok(file) => {
                                                let bajs = current_path.clone();
                                                new_state =
                                                    Some(Box::new(|_| CurrentUI::LoadingApp {
                                                        file,
                                                        file_path: bajs,
                                                    }));
                                            }
                                            Err(_) => (),
                                        }
                                    } else if item.3 == COLOR_MUSIC {
                                        current_path.push_str(&item.1);
                                        match fatfs_embedded::open(current_path, FileOptions::Read)
                                        {
                                            Ok(mut module) => {
                                                let bajs = current_path.clone();
                                                new_state =
                                                    Some(Box::new(|_| CurrentUI::LoadingMusic {
                                                        file: module,
                                                        file_path: bajs,
                                                    }));
                                            }
                                            Err(_abort) => pop_dir_entry(current_path),
                                        }
                                    }
                                }
                            }
                        }
                        if ui.input_pressed(gui::Input(Buttons::BUTTON_B)) && new_folder.is_none() {
                            if current_path != "sd:/" && current_path != "nand:/" {
                                pop_dir_entry(current_path);
                                if let Ok(f) = fatfs_embedded::opendir(current_path) {
                                    new_folder = Some(f);
                                }
                            } else {
                                new_state = Some(Box::new(|_| CurrentUI::None));
                            }
                        }
                        if let Some(mut new_folder) = new_folder {
                            *immediate_files = populate_fs_vec(&mut new_folder);
                        }

                        new_state
                    }
                };
            if let Some(new_state) = new_state_fn {
                let mut fuck_off = CurrentUI::None;
                core::mem::swap(&mut fuck_off, &mut self.current_dir);
                self.current_dir = new_state(fuck_off);
            }
        });
    }
}
fn pop_dir_entry(current_path: &mut String) {
    current_path.pop();
    while current_path.pop() != Some('/') {}
    current_path.push('/');
}

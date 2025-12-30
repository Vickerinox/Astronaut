use crate::{
    boot, gui, is_bootable, is_music_module, populate_fs_vec, send_mod_file, stop_mod_file,
    COLOR_BOOTABLE, COLOR_MUSIC,
};
use alloc::{
    format,
    string::{String, ToString},
    vec::{self, Vec},
};
use fatfs_embedded::fatfs::{File, FileInfo, FileOptions, FS_SD};
use micro_imgui::{widgets::button::Button, Color, Sizing, Vec2};
use reboot_lib::{
    music_modules::mods::{MODAsyncLoader, MODHeader},
    Buttons,
};

enum CurrentDirectory {
    None,
    NAND {
        immediate_files: Vec<(String, bool, Color)>,
        file_path: String,
    },
    SD {
        immediate_files: Vec<(String, bool, Color)>,
        file_path: String,
    },
}
enum LoadingFile {
    None,
    App {
        file: fatfs_embedded::fatfs::File,
    },
    Music {
        file: fatfs_embedded::fatfs::File,
        data: Vec<u8>,
    },
}

pub struct AppData{
    current_dir: CurrentDirectory,
    loading_file: LoadingFile,
    loading_mod_file: Option<MODAsyncLoader>,
}
impl CurrentDirectory {
    pub fn is_in_sd(&self) -> bool {
        if let Self::SD { .. } = self {
            true
        } else {
            false
        }
    }
    pub fn is_in_nand(&self) -> bool {
        if let Self::NAND { .. } = self {
            true
        } else {
            false
        }
    }
}
impl AppData {
    pub fn new() -> Self {
        Self {
            current_dir: CurrentDirectory::None,
            loading_file: LoadingFile::None,
            loading_mod_file: None,
        }
    }
    pub fn open_sd() -> Option<CurrentDirectory> {
        let mut file_path = String::from("sd:/");
        fatfs_embedded::opendir(&mut file_path).ok().map(|mut i| {
            let immediate_files = populate_fs_vec(&mut i);
            CurrentDirectory::SD {
                immediate_files,
                file_path,
            }
        })
    }
    pub fn open_nand() -> Option<CurrentDirectory> {
        let mut file_path = String::from("nand:/");
        fatfs_embedded::opendir(&mut file_path).ok().map(|mut i| {
            let immediate_files = populate_fs_vec(&mut i);
            CurrentDirectory::NAND {
                immediate_files,
                file_path,
            }
        })
    }
    pub fn open_default_fs(&mut self) {
        if let Some(dir) = Self::open_sd() {
            self.current_dir = dir;
        } else if let Some(dir) = Self::open_nand() {
            self.current_dir = dir;
        }
    }
    pub fn play_startup_music(&mut self) {
        match fatfs_embedded::open(
            &mut "sd:/_nds/vlaunch/default.bin".to_string(),
            FileOptions::Read,
        ) {
            Ok(mut file) => {
                let mut size = [0u8; 2];
                if fatfs_embedded::read(&mut file, &mut size).is_err() {
                    return;
                }
                let size = u16::from_le_bytes(size) as usize;
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
        f.central_panel(|ui| {
            if ui.input_pressed(gui::Input(Buttons::BUTTON_L))
                || ui.input_pressed(gui::Input(Buttons::BUTTON_R))
            {
                if self.current_dir.is_in_nand() {
                    if let Some(dir) = AppData::open_sd() {
                        self.current_dir = dir;
                    }
                } else if self.current_dir.is_in_sd() {
                    if let Some(dir) = AppData::open_nand() {
                        self.current_dir = dir;
                    }
                }
            }

            let heading = match &self.current_dir {
                CurrentDirectory::None => "Unable To Mount FS!!!",
                CurrentDirectory::NAND { .. } => "NAND view:",
                CurrentDirectory::SD { .. } => "SD Card view:",
            };
            ui.header(heading);

            let mut new_folder = None;

            match &mut self.current_dir {
                CurrentDirectory::None => (),
                CurrentDirectory::NAND {
                    immediate_files,
                    file_path: current_path,
                }
                | CurrentDirectory::SD {
                    immediate_files,
                    file_path: current_path,
                } => {
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
                        ui.label(&current_path);
                    }

                    let mut a = LoadingFile::None;
                    core::mem::swap(&mut a, &mut self.loading_file);
                    match a {
                        LoadingFile::None => {
                            for item in immediate_files.iter() {
                                if ui
                                    .add(Button::new(
                                        &item.0,
                                        Sizing::Padded(Vec2::new(248, 8)),
                                        item.2,
                                    ))
                                    .clicked()
                                {
                                    if item.1 {
                                        current_path.push_str(&item.0);
                                        current_path.push('/');
                                        if let Ok(f) = fatfs_embedded::opendir(current_path) {
                                            new_folder = Some(f);
                                        }
                                    } else {
                                        if item.2 == COLOR_BOOTABLE {
                                            current_path.push_str(&item.0);
                                            match fatfs_embedded::open(
                                                current_path,
                                                FileOptions::Read,
                                            ) {
                                                Ok(file) => {
                                                    unsafe {
                                                        boot::boot_app(file, &current_path).unwrap()
                                                    };
                                                }
                                                Err(_) => (),
                                            }
                                        } else if item.2 == COLOR_MUSIC {
                                            current_path.push_str(&item.0);
                                            match fatfs_embedded::open(
                                                current_path,
                                                FileOptions::Read,
                                            ) {
                                                Ok(mut module) => {
                                                    let mut header_buffer = alloc::vec![0u8; 0x640];
                                                    //module.read_exact(&mut header_buffer);
                                                    self.loading_file = LoadingFile::Music {
                                                        file: module,
                                                        data: header_buffer,
                                                    };
                                                }
                                                Err(_abort) => pop_dir_entry(current_path),
                                            }
                                        }
                                    }
                                }
                            }
                            if ui.input_pressed(gui::Input(Buttons::BUTTON_B))
                                && new_folder.is_none()
                            {
                                if current_path != "sd:/" && current_path != "nand:/" {
                                    pop_dir_entry(current_path);
                                    if let Ok(f) = fatfs_embedded::opendir(current_path) {
                                        new_folder = Some(f);
                                    }
                                }
                            }
                            if let Some(mut new_folder) = new_folder {
                                *immediate_files = populate_fs_vec(&mut new_folder);
                            }
                        }
                        LoadingFile::App { file } => {}
                        LoadingFile::Music { mut file, data } => {
                            if ui.button("Play song").clicked() {
                                if fatfs_embedded::seek(&mut file, 0) == Ok(()) {
                                    self.loading_mod_file = Some(MODAsyncLoader::new(file));
                                    drop(stop_mod_file());
                                }
                                pop_dir_entry(current_path);
                            } else if ui.button("go back").clicked() {
                                self.loading_file = LoadingFile::None;
                                pop_dir_entry(current_path);
                            } else if ui.button("set default").clicked() {
                                let mut file = match fatfs_embedded::open(
                                    &mut "sd:/_nds/vLaunch/default.bin".to_string(),
                                    FileOptions::Write | FileOptions::CreateAlways,
                                ) {
                                    Ok(file) => file,
                                    Err(what) => panic!("{:?}", what),
                                };

                                if current_path.len() < 1000 {
                                    match fatfs_embedded::write(
                                        &mut file,
                                        &(current_path.len() as u16).to_le_bytes(),
                                    ) {
                                        Ok(2) => (),
                                        _ => panic!(),
                                    };
                                    let bytes = current_path.as_bytes();
                                    match fatfs_embedded::write(&mut file, bytes) {
                                        Ok(len) => assert_eq!(len as usize, bytes.len()),
                                        _ => panic!(),
                                    };
                                    fatfs_embedded::truncate(&mut file).unwrap();
                                    unsafe { FS_SD.sync(&mut file).unwrap() };
                                }

                                pop_dir_entry(current_path);
                            } else {
                                self.loading_file = LoadingFile::Music { file, data };
                            }
                        }
                    }
                }
            }
        });
    }
}
fn pop_dir_entry(current_path: &mut String) {
    current_path.pop();
    while current_path.pop() != Some('/') {}
    current_path.push('/');
}

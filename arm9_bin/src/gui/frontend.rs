use alloc::{format, string::String, vec::Vec};
use common::bootstrap::HeaderTWL;
use reboot_lib::{Buttons, fatfs::{Dir, File, FileSystem, LossyOemCpConverter, NullTimeProvider, OemCpConverter, Read, ReadWriteSeek, Seek, SeekFrom, TimeProvider}, music_modules::mods::{MODAsyncLoader, MODHeader}};

use crate::{bootloader, gui, is_bootable, is_music_module, populate_fs_vec, send_mod_file, start_procedural_music, stop_mod_file};
use micro_imgui::{Color, Sizing, Vec2, widgets::button::Button};

enum CurrentDirectory<'a, T: ReadWriteSeek, TP, OCC> {
    None,
    NAND {
        current_dir: Dir<'a, T, TP, OCC>,
        immediate_files: Vec<(String, bool, Color)>,
        file_path: String,
    },
    SD {
        current_dir: Dir<'a, T, TP, OCC>,
        immediate_files: Vec<(String, bool, Color)>,
        file_path: String,
    },
}
enum LoadingFile<'a, T: ReadWriteSeek, TP, OCC> {
    None,
    App {
        file: File<'a, T, TP, OCC>,
        data: Vec<u8>,
    },
    Music {
        file: File<'a, T, TP, OCC>,
        data: Vec<u8>,
    },
}

pub struct AppData<'a, T: ReadWriteSeek, TP: TimeProvider = NullTimeProvider, OCC = LossyOemCpConverter> {
    current_dir: CurrentDirectory<'a, T, TP, OCC>,
    loading_file: LoadingFile<'a, T, TP, OCC>,
    loading_mod_file: Option<MODAsyncLoader<File<'a, T, TP, OCC>>>
}
impl<'a, T: ReadWriteSeek, TP, OCC> CurrentDirectory<'a, T, TP, OCC> {
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
impl<'a, T: ReadWriteSeek, TP: TimeProvider, OCC: OemCpConverter> AppData<'a, T, TP, OCC> {
    pub fn new() -> Self {
        Self { current_dir: CurrentDirectory::None, loading_file: LoadingFile::None, loading_mod_file: None }
    }
    pub fn open_default_fs(&mut self, nand_fs: &'a Option<FileSystem<T, TP, OCC>>, sd_fs: &'a Option<FileSystem<T, TP, OCC>>) {
        if let Some(sd_fs) = sd_fs {
            let current_dir = sd_fs.root_dir();
            let immediate_files = populate_fs_vec(&current_dir);
            let file_path = String::from("sd:/");
            self.current_dir = CurrentDirectory::SD { current_dir, immediate_files, file_path };
        } else if let Some(nand_fs) = nand_fs {
            let current_dir = nand_fs.root_dir();
            let immediate_files = populate_fs_vec(&current_dir);
            let file_path = String::from("nand:/");
            self.current_dir = CurrentDirectory::NAND { current_dir, immediate_files, file_path };
        }
    }
    pub fn play_startup_music(&mut self, sd_fs: &'a Option<FileSystem<T, TP, OCC>>) {
        
        if let Some(folder) = sd_fs.as_ref() {
            let root = folder.root_dir();

            match root.open_file("/_nds/vlaunch/default.mod") {
                Ok(file) => {
                    stop_mod_file();
                    self.loading_mod_file = Some(MODAsyncLoader::new(file));
                }
                Err(_abort) => {
                    start_procedural_music();
                }
            }
        } else {
            start_procedural_music();
        }
        
    }
    pub fn update(&mut self, f: &mut micro_imgui::Frame<'_, super::DSMicroGuiBackend>, nand_fs: &'a Option<FileSystem<T, TP, OCC>>, sd_fs: &'a Option<FileSystem<T, TP, OCC>>) {
            f.central_panel(|ui| {
                if ui.input_pressed(gui::Input(Buttons::BUTTON_L)) || ui.input_pressed(gui::Input(Buttons::BUTTON_R)) {
                    if !self.current_dir.is_in_nand() && nand_fs.is_some() {
                        if let Some(root) = nand_fs.as_ref() {
                            let current_dir = root.root_dir();
                            let immediate_files = populate_fs_vec(&current_dir);
                            let file_path = String::from("nand:/");
                            self.current_dir = CurrentDirectory::NAND { current_dir, immediate_files, file_path };
                        }       
                    } else if !self.current_dir.is_in_sd() && sd_fs.is_some() {
                        if let Some(root) = sd_fs.as_ref() {
                            let current_dir = root.root_dir();
                            let immediate_files = populate_fs_vec(&current_dir);
                            let file_path = String::from("sd:/");
                            self.current_dir = CurrentDirectory::SD { current_dir, immediate_files, file_path };
                        } 
                    }
                }
                let heading = match &self.current_dir {
                    CurrentDirectory::None => {
                        ui.header("FS ERROR");
                        ui.label("no file system could be mounted. (for some reason not even nand?)");
                        return;
                    },
                    CurrentDirectory::NAND { .. } => "NAND view:",
                    CurrentDirectory::SD { .. } => "SD Card view:",
                };
                ui.header(heading);

                let mut new_folder = None;

                match &mut self.current_dir {
                    CurrentDirectory::None => (),
                    CurrentDirectory::NAND { current_dir: working_folder, immediate_files: folders, file_path: current_path } |
                    CurrentDirectory::SD { current_dir: working_folder, immediate_files: folders, file_path: current_path } => {
                        if let Some(loading_mod) = self.loading_mod_file.take() {
                        let (progress, max) = loading_mod.progress();
                        let progress_bar = progress * 27 / max;
                        let bar = (0..27)
                            .map(|i| if i < progress_bar { '=' } else { '.' })
                            .collect::<String>();
                        ui.label(format!("Loading song [{bar}]"));
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
                        LoadingFile::None =>  {
                            for item in folders.iter() {
                            if ui
                                .add(Button::new(&item.0, Sizing::Padded(Vec2::new(248, 8)), item.2))
                                .clicked()
                            {
                                if item.1 {
                                    match working_folder.open_dir(&item.0) {
                                        Ok(folder) => {
                                            if &item.0 == "." {}
                                            else if &item.0 == ".." {
                                                current_path.pop();
                                                while current_path.pop() != Some('/') {}
                                                current_path.push('/');
                                            }
                                            else {
                                                current_path.push_str(&item.0);
                                                current_path.push('/');
                                            }
                                            new_folder = Some(folder)
                                        },
                                        Err(_) => (),
                                    }
                                } else {
                                    let extension_point = item.0.len() - 4;
                                    if item.0.is_char_boundary(extension_point) {
                                        if is_bootable(item.0.as_bytes()) {
                                            match working_folder.open_file(&item.0) {
                                                Ok(mut file) => {
                                                    current_path.push_str(&item.0);
                                                    let mut header_buffer = alloc::vec![0u8; 4096];
                                                    file.read(&mut header_buffer);
                                                    self.loading_file = LoadingFile::App { file, data: header_buffer };
                                                }
                                                Err(_) => (),
                                            }
                                        } else if is_music_module(item.0.as_bytes()) {
                                            match working_folder.open_file(&item.0) {
                                                Ok(mut module) => {
                                                    let mut header_buffer = alloc::vec![0u8; 0x640];
                                                    module.read(&mut header_buffer);
                                                    self.loading_file = LoadingFile::Music { file: module, data: header_buffer };
                                                    
                                                }
                                                Err(_abort) => (),
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if let Some(new_folder) = new_folder {
                            *folders = populate_fs_vec(&new_folder);
                            *working_folder = new_folder;
                        }
                        },
                        LoadingFile::App { mut file, mut data } => {
                            let head = unsafe { &mut *(&mut data[..] as *mut [u8] as *mut u8
                            as *mut HeaderTWL) };
                        ui.label("                 Title info:");
                        ui.label(alloc::format!(
                            "      Name: {} TID: {:08X}",
                            core::str::from_utf8(&head.title).unwrap_or("UNKNOWN"),
                            head.tid
                        ));
                        ui.label(" ");
                        ui.label("ARM9 offsets:");
                        ui.label(alloc::format!("entry: {:08X}", head.arm9_entry));
                        ui.label(alloc::format!(
                            "load: {:08X}, size: {:08X}",
                            head.arm9_load,
                            head.arm9_size
                        ));
                        ui.label(" ");
                        ui.label("ARM7 offsets:");
                        ui.label(alloc::format!("entry: {:08X}", head.arm7_entry));
                        ui.label(alloc::format!(
                            "load: {:08X}, size: {:08X}",
                            head.arm7_load,
                            head.arm7_size
                        ));
                        ui.label("ARMi offsets:");
                        ui.label(alloc::format!(
                            "7i: {:08X} {:08X}",
                            head.arm7i_load,
                            head.arm7i_size
                        ));
                        ui.label(alloc::format!(
                            "9i: {:08X} {:08X}",
                            head.arm9i_load,
                            head.arm9i_size
                        ));
                        ui.label(" ");
                        
                        ui.label(alloc::format!(
                            "mbk: {:08X} {:08X} {:08X} {:08X} {:08X} | {:08X} {:08X} {:08X} | {:08X} {:08X} {:08X} ",
                            head.global_mbks[0], head.global_mbks[1], head.global_mbks[2],
                            head.global_mbks[3], head.global_mbks[4],
                            head.arm9_mbks[0], head.arm9_mbks[1], head.arm9_mbks[2],
                            head.arm7_mbks[0], head.arm7_mbks[1], head.arm7_mbks[2],
                        ));
                        ui.label(" ");
                        ui.horizontal(|ui| {
                            if ui.button("Launch!!").clicked() {
                                
                                match file.seek(SeekFrom::Start(0)) {
                                    Ok(0) => {
                                        unsafe { bootloader::boot_app(file, &current_path); }
                                    }
                                    Ok(_what) => (),
                                    Err(_error) => (),
                                }
                            } else {
                                if ui.button("Go back").clicked() {
                                    while current_path.pop() != Some('/') {}
                                    current_path.push('/');
                                    self.loading_file = LoadingFile::None;
                                } else {
                                    self.loading_file = LoadingFile::App { file, data };
                                }
                            }
                        });
                        }
                        LoadingFile::Music { mut file, mut data } =>  {
                            
                            let head = unsafe { &mut *(&mut data[..] as *mut [u8] as *mut u8
                            as *mut MODHeader) };
                            let song_name = str::from_utf8(&head.song_name).unwrap_or("UNKNOWN");
                            ui.label(song_name);
                            
                            if ui.button("Play song").clicked() {
                                if file.seek(SeekFrom::Start(0)).ok() == Some(0) {
                                    self.loading_mod_file = Some(MODAsyncLoader::new(file));
                                    drop(stop_mod_file());
                                }
                            } else if ui.button("go back").clicked() {
                                self.loading_file = LoadingFile::None;
                            } else {
                                self.loading_file = LoadingFile::Music { file, data };
                            }
                        }
                    }
                    },
                }
            });
    }
}
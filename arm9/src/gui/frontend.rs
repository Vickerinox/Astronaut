use alloc::{format, string::String, vec::Vec};
use reboot_lib::{
    music_modules::mods::{MODAsyncLoader, MODHeader},
    Buttons,
};
use fatfs_embedded::fatfs::FS;
use crate::{
    boot, gui, is_bootable, is_music_module, populate_fs_vec, send_mod_file, stop_mod_file,
};
use micro_imgui::{widgets::button::Button, Color, Sizing, Vec2};

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
enum LoadingFile<'a, T: reboot_lib::fatfs::ReadWriteSeek, TP, OCC> {
    None,
    App {
        file: reboot_lib::fatfs::File<'a, T, TP, OCC>,
    },
    Music {
        file: reboot_lib::fatfs::File<'a, T, TP, OCC>,
        data: Vec<u8>,
    },
}

pub struct AppData/*<
    'a,
    T: ReadWriteSeek,
    TP: TimeProvider = NullTimeProvider,
    OCC = LossyOemCpConverter,
>*/ {
    current_dir: CurrentDirectory,
    //loading_file: LoadingFile<'a, T, TP, OCC>,
    //loading_mod_file: Option<MODAsyncLoader<File<'a, T, TP, OCC>>>,
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
            current_dir: CurrentDirectory::None
        }
    }
    pub fn open_default_fs(
        &mut self
    ) {
        if let Ok(mut sd_fs) =  unsafe { FS.opendir("sd:/") } {
            let immediate_files = populate_fs_vec(&mut sd_fs);
            let file_path = String::from("sd:/");
            unsafe { FS.closedir(&mut sd_fs) };
            self.current_dir = CurrentDirectory::SD {
                immediate_files,
                file_path,
            };
        } else if let Ok(mut nand_fs) =  unsafe { FS.opendir("nand:/") } {
            let immediate_files = populate_fs_vec(&mut nand_fs);
            let file_path = String::from("nand:/");
            unsafe { FS.closedir(&mut nand_fs) };
            self.current_dir = CurrentDirectory::NAND {
                immediate_files,
                file_path,
            };
        }
    }
    pub fn play_startup_music(&mut self) {
        /* 
        if let Some(folder) = sd_fs.as_ref() {
            let root = folder.root_dir();

            match root.open_file("/_nds/vlaunch/default.bin") {
                Ok(mut file) => {
                    stop_mod_file();
                    let mut size = [0u8; 2];
                    if file.read_exact(&mut size).is_err() {
                        //start_procedural_music();
                        return;
                    }
                    let size = u16::from_le_bytes(size) as usize;
                    let mut path_buf: Vec<u8> = alloc::vec![0; size];
                    if file.read_exact(&mut path_buf).is_err() {
                        //start_procedural_music();
                        return;
                    }
                    let Ok(str) = core::str::from_utf8(&path_buf) else {
                        //start_procedural_music();
                        return;
                    };
                    let Ok(file) = root.open_file(str) else {
                        return;
                    };
                    self.loading_mod_file = Some(MODAsyncLoader::new(file));
                    return;
                }
                Err(_abort) => {}
            }
        }
        */
    }
    pub fn update(
        &mut self,
        f: &mut micro_imgui::Frame<'_, super::DSMicroGuiBackend>
    ) {
        f.central_panel(|ui| {
            /* 
            if ui.input_pressed(gui::Input(Buttons::BUTTON_L))
                || ui.input_pressed(gui::Input(Buttons::BUTTON_R))
            {
                if !self.current_dir.is_in_nand() && nand_fs.is_some() {
                    if let Some(root) = nand_fs.as_ref() {
                        let current_dir = root.root_dir();
                        let immediate_files = populate_fs_vec(&current_dir);
                        let file_path = String::from("nand:/");
                        self.current_dir = CurrentDirectory::NAND {
                            immediate_files,
                            file_path,
                        };
                    }
                } else if !self.current_dir.is_in_sd() && sd_fs.is_some() {
                    if let Some(root) = sd_fs.as_ref() {
                        let current_dir = root.root_dir();
                        let immediate_files = populate_fs_vec(&current_dir);
                        let file_path = String::from("sd:/");
                        self.current_dir = CurrentDirectory::SD {
                            immediate_files,
                            file_path,
                        };
                    }
                }
            }
            */
            let heading = match &self.current_dir {
                CurrentDirectory::None => "",
                CurrentDirectory::NAND { .. } => "NAND view:",
                CurrentDirectory::SD { .. } => "SD Card view:",
            };
            ui.header(heading);

            //let mut new_folder = None;

            match &mut self.current_dir {
                CurrentDirectory::None => (),
                CurrentDirectory::NAND {
                    immediate_files,
                    file_path: current_path,
                }
                | CurrentDirectory::SD {
                    immediate_files,
                    file_path: current_path,
                } => { /* 
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
                    */


                    /* 
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
                                        
                                    } else {
                                        let extension_point = item.0.len() - 4;
                                        if item.0.is_char_boundary(extension_point) {
                                            if is_bootable(item.0.as_bytes()) {
                                                match current_dir.open_file(&item.0) {
                                                    Ok(mut file) => {
                                                        current_path.push_str(&item.0);
                                                        match file.seek(SeekFrom::Start(0)) {
                                                            Ok(0) => unsafe {
                                                                //boot::boot_app(file, &current_path)
                                                                //    .expect("failed app boot");
                                                            },

                                                            Ok(_what) => (),
                                                            Err(_error) => (),
                                                        }
                                                    }
                                                    Err(_) => (),
                                                }
                                            } else if is_music_module(item.0.as_bytes()) {
                                                match current_dir.open_file(&item.0) {
                                                    Ok(mut module) => {
                                                        current_path.push_str(&item.0);
                                                        let mut header_buffer =
                                                            alloc::vec![0u8; 0x640];
                                                        module.read_exact(&mut header_buffer);
                                                        self.loading_file = LoadingFile::Music {
                                                            file: module,
                                                            data: header_buffer,
                                                        };
                                                    }
                                                    Err(_abort) => (),
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            if let Some(new_folder) = new_folder {
                                *immediate_files = populate_fs_vec(&new_folder);
                                *current_dir = new_folder;
                            }
                        }
                        LoadingFile::App { file } => {}
                        LoadingFile::Music { mut file, data } => {
                            if ui.button("Play song").clicked() {
                                if file.seek(SeekFrom::Start(0)).ok() == Some(0) {
                                    self.loading_mod_file = Some(MODAsyncLoader::new(file));
                                    drop(stop_mod_file());
                                }
                                pop_dir_entry(current_path);
                            } else if ui.button("go back").clicked() {
                                self.loading_file = LoadingFile::None;
                                pop_dir_entry(current_path);
                            } else if ui.button("set default").clicked() {
                                let Some(fs) = &sd_fs else { return };
                                let root = fs.root_dir();
                                let Ok(mut file) = root.create_file("/_nds/vLaunch/default.bin")
                                else {
                                    return;
                                };

                                if current_path.len() < 1000 {
                                    let Some(slice) = current_path.get(3..) else {
                                        return;
                                    };
                                    file.write_all(&(slice.len() as u16).to_le_bytes());
                                    file.write_all(slice.as_bytes());
                                    file.truncate();
                                    file.flush();
                                }
                                pop_dir_entry(current_path);
                            } else {
                                self.loading_file = LoadingFile::Music { file, data };
                            }
                        }
                    }
                    */
                }
            }
        });
    }
}
fn pop_dir_entry(current_path: &mut alloc::string::String) {
    while current_path.pop() != Some('/') {}
    current_path.push('/');
}

use crate::{
    boot,
    gui::{self, Input},
    populate_fs_vec, send_mod_file, stop_mod_file, AppArea, APP_AREA_START, BACKGROUND_COLOR,
    COLOR_BOOTABLE, COLOR_MUSIC, SCREEN_RECT,
};
use alloc::{
    boxed::Box,
    format,
    string::{String, ToString},
    vec::Vec,
};
use common::blowfish::BFCTX;
use fatfs_embedded::fatfs::{FileOptions, RawFileSystem};
use micro_imgui_ds::micro_imgui::{self, widgets::checkbox::Checkbox};
use micro_imgui_ds::micro_imgui::{widgets::button::Button, Backend, Color, Sizing, Vec2};
use reboot_lib::{
    autoboot_info::{UnlaunchParams, BOOT_INFO},
    music_modules::mods::MODAsyncLoader,
    Buttons, VIDEO_HARDWARE,
};
pub enum CurrentUI {
    None,
    Error {
        error_string: String,
    },
    Browsing {
        immediate_files: Vec<(String, String, bool, Color)>,
        file_path: String,
        offset: i32,
        drag_start: i16,
    },
    LoadingApp {
        file: fatfs_embedded::fatfs::File,
        file_path: String,
    },
    LoadingMusic {
        file: fatfs_embedded::fatfs::File,
        file_path: String,
    },
    SpecialThanks,
}
pub struct AppData {
    pub autoboot: Option<(String, &'static UnlaunchParams)>,
    pub current_ui: CurrentUI,
    pub blowfish: BFCTX,
    pub loading_mod_file: Option<MODAsyncLoader>,
    pub nand_fs: RawFileSystem,
    pub sdmc_fs: RawFileSystem,
    pub patch_flag: bool,
}
pub struct FileEntry {
    filename: String,
    truncated_name: String,
    file_type: FileType,
}
pub enum FileType {}
impl AppData {
    pub fn open_sd() -> Option<CurrentUI> {
        let mut file_path = String::from("sdmc:/");
        fatfs_embedded::opendir(&mut file_path).ok().map(|mut i| {
            let immediate_files = populate_fs_vec(&mut i);
            CurrentUI::Browsing {
                immediate_files,
                file_path,
                offset: 0,
                drag_start: 0,
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
                offset: 0,
                drag_start: 0,
            }
        })
    }
    pub unsafe fn autoboot(&mut self) {
        match fatfs_embedded::open(
            &mut "sdmc:/_nds/vlaunch/autoboot.txt".to_string(),
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
                (*(APP_AREA_START as *mut AppArea)).fader.target.write(16);
                //self.current_ui = CurrentUI::LoadingApp { file, file_path: str };
                crate::boot::boot_app(&mut file, &str, self);
            }
            Err(_abort) => {}
        }
    }
    pub fn play_startup_music(&mut self) {
        match fatfs_embedded::open(
            &mut "sdmc:/_nds/vlaunch/music.bin".to_string(),
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
        let _mouse = f.last_known_pointer_location();
        f.central_panel(|ui| {
            {
                let color = BACKGROUND_COLOR;
                ui.paint_shape(micro_imgui::Shape::Rectangle {
                    area: SCREEN_RECT.include_point(Vec2::new(256, 256)),
                    fill: Color(color),
                    rounding: 0,
                    outline_color: Color(color),
                    outline_size: 0,
                });
            }

            unsafe {
                let sdio = reboot_lib::twl_wifi::STATUS.read_volatile();
                ui.label(&format!("SDIO: {:08x?}", sdio));
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

            let new_state_fn: Option<Box<dyn FnOnce(CurrentUI) -> CurrentUI>> = match &mut self
                .current_ui
            {
                CurrentUI::Error { error_string } => {
                    ui.header("ERROR:");
                    ui.label(error_string);
                    if ui.button("okay").clicked() {
                        Some(Box::new(|_| CurrentUI::None))
                    } else {
                        None
                    }
                }
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
                    ui.add(Checkbox::new(&mut self.patch_flag, "Enable patching"));
                    if ui.input_pressed(gui::Input(Buttons::BUTTON_START)) {
                        res = Some(Box::new(|_| CurrentUI::SpecialThanks));
                    }
                    ui.add_space(82);
                    ui.label(concat!("build commit: ",env!("GIT_HASH")));
                    res
                }
                CurrentUI::LoadingApp { file, file_path } => {
                    ui.request_repaint();
                    
                    let mut swap = CurrentUI::None;
                    core::mem::swap(&mut self.current_ui, &mut swap);
                    if let CurrentUI::LoadingApp { mut file, file_path } = swap {
                        let error = unsafe {
                        (*(APP_AREA_START as *mut AppArea)).fader.target.write(16);
                        boot::boot_app(&mut file, &file_path, self)
                        };
                        unsafe { (*(APP_AREA_START as *mut AppArea)).fader.target.write(0) };
                        let error_string = alloc::format!("Failed to boot file: {error:?}");
                        self.current_ui = CurrentUI::Error { error_string };
                    
                    }
                    
                    None
                }
                CurrentUI::LoadingMusic { file, file_path } => {
                    if ui.button("Play song").clicked() {
                        if fatfs_embedded::seek(file, 0) == Ok(()) {
                            Some(Box::new(|s| {
                                if let CurrentUI::LoadingMusic { file, file_path: _ } = s {
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
                            &mut "sdmc:/_nds/vLaunch/music.bin".to_string(),
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

                            self.sdmc_fs.sync(&mut file).unwrap()
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
                    offset,
                    drag_start,
                } => {
                    const ITEM_SPACING: i32 = 14;

                    if let Some(drag) = ui.drag() {
                        let new_drag = drag.y - *drag_start;
                        *drag_start += new_drag;
                        *offset -= new_drag as i32;
                        *offset = (*offset)
                            .min(((immediate_files.len() * 14) as i32) - (ITEM_SPACING * 10))
                            .max(0);
                    } else {
                        *drag_start = 0;
                    }

                    let shown_items = immediate_files
                        .get(((*offset / ITEM_SPACING) as usize)..)
                        .unwrap_or(&[]);

                    let in_step = *offset % ITEM_SPACING;

                    let mut control_dir = 0;
                    if ui.input_pressed(Input(Buttons::DIRECTION_UP)) {
                        control_dir = 1;
                    }

                    if ui.input_pressed(Input(Buttons::DIRECTION_DOWN)) {
                        
                        control_dir = -1;
                    }

                    let mut new_state: Option<
                        alloc::boxed::Box<dyn FnOnce(CurrentUI) -> CurrentUI>,
                    > = None;
                    let mut new_folder = None;
                    ui.label(current_path);
                    let rect = micro_imgui::Rect::from_two_pos(
                        ui.clip_rect().top_left(),
                        ui.clip_rect().top_right() + Vec2::new(0, ITEM_SPACING as _),
                    );
                    let rect2 = micro_imgui::Rect::from_two_pos(
                        SCREEN_RECT.bottom_left() - Vec2::new(0, 8 as _),
                        SCREEN_RECT.bottom_right() + Vec2::new(0, 1),
                    );

                    let color = BACKGROUND_COLOR;

                    ui.add_space((ITEM_SPACING - in_step) as i16);
                    let items = if in_step == 0 { 10 } else { 11 };
                    let mut focus = None;
                    for (i, item) in shown_items.iter().take(items).enumerate() {
                        let response = ui
                            .add(Button::new(
                                &item.0,
                                Sizing::Padded(Vec2::new(248, 8)),
                                item.3,
                            ));
                        if response.focused() {
                            focus = Some(i);
                        }
                        if response.clicked()
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
                                    match fatfs_embedded::open(current_path, FileOptions::Read) {
                                        Ok(file) => {
                                            let bajs = current_path.clone();
                                            new_state = Some(Box::new(|_| CurrentUI::LoadingApp {
                                                file,
                                                file_path: bajs,
                                            }));
                                        }
                                        Err(_) => (),
                                    }
                                } else if item.3 == COLOR_MUSIC {
                                    current_path.push_str(&item.1);
                                    match fatfs_embedded::open(current_path, FileOptions::Read) {
                                        Ok(module) => {
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
                    ui.paint_shape(micro_imgui::Shape::Rectangle {
                        area: rect,
                        fill: Color(color),
                        rounding: 0,
                        outline_color: Color(color),
                        outline_size: 0,
                    });
                    ui.paint_shape(micro_imgui::Shape::Rectangle {
                        area: rect2,
                        fill: Color(color),
                        rounding: 0,
                        outline_color: Color(color),
                        outline_size: 0,
                    });

                    if control_dir == 1 {
                        if focus == Some(0) && *offset > 0 {
                            *offset = offset.wrapping_sub(ITEM_SPACING).max(0);
                            ui.cancel_refocus();
                        }
                        
                        *offset -= in_step;
                        
                    }
                    else if control_dir == -1 {
                        if focus == Some(9) && shown_items.len() >= 11 {
                            *offset = offset.saturating_add(ITEM_SPACING);
                            ui.cancel_refocus();
                        }
                        
                        *offset -= in_step;
                        
                    }
                    if ui.input_pressed(gui::Input(Buttons::BUTTON_B)) && new_folder.is_none() {
                        if ["nand:/","sdmc:/"].contains(&current_path.as_str()) {
                            new_state = Some(Box::new(|_| CurrentUI::None));
                        } else {
                            pop_dir_entry(current_path);
                            match fatfs_embedded::opendir(current_path) {
                               Ok(f) => {new_folder = Some(f);}
                               Err(_err) => {new_state = Some(Box::new(|_| CurrentUI::Error { error_string: format!("Filesystem error!") }));}
                            }
                        }
                    }
                    if let Some(mut new_folder) = new_folder {
                        *immediate_files = populate_fs_vec(&mut new_folder);
                        *offset = 0;
                    }
                    new_state
                }
                CurrentUI::SpecialThanks => {
                    ui.header("Special thanks");
                    let names = &[
                        "edo9300",
                        "nocash",
                        "Team LNH",
                        "f3l1x_10m",
                        "coderkei",
                        "rmc",
                        "PoroCYon",
                        "AntonioND",
                        "and you!",        
                    ];
                    for name in names {
                        ui.label(name);
                    }
                    
                    if ui.input_pressed(gui::Input(Buttons::BUTTON_B)) {
                        Some(Box::new(|_| CurrentUI::None))
                    } else {
                        None
                    }
                },
            };
            if let Some(new_state) = new_state_fn {
                let mut current_ui = CurrentUI::None;
                core::mem::swap(&mut current_ui, &mut self.current_ui);
                self.current_ui = new_state(current_ui);
            }
        });
    }
}
fn pop_dir_entry(current_path: &mut String) {
    current_path.pop();
    while current_path.pop() != Some('/') {}
    current_path.push('/');
}

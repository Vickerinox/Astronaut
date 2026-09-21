// SPDX-FileCopyrightText: 2026 Viktor Karlsson <viktor@koda.re>
// SPDX-License-Identifier: GPL-3.0-or-later

pub mod panels;

use std::{error::Error, fs, path::PathBuf, sync::Arc};

use build_tools::DecodedBMP;
use eframe::{NativeOptions, egui::{self, Color32, RichText, Sense, TextureHandle, Vec2}, wgpu::naga::CooperativeRole::A};
use egui_dock::{LeafNode, NodePath};

use crate::panels::{ColorPalettePanel, Panel, PanelType, PreviewControlsPanel, PreviewPanel};

fn read_bmp(path: PathBuf) -> Result<DecodedBMP, Box<dyn Error>> {
    let file = fs::OpenOptions::new()
        .read(true)
        .open(path)
        .map_err(|e| format!("Error opening file: {e}"))?;
    let bmp = build_tools::DecodedBMP::from_reader(file)
        .map_err(|e| format!("Error decoding BMP {e}"))?;
    if bmp.width() != 1024
        || bmp.height() != 8
        || bmp.dib.compression != 0
        || bmp.dib.bits_per_pixel != 4
    {
        Err(format!("Bitmap must be 1024x8 pixels, use no compression, and use 4-bit color. Yours is {}x{}, uses compresion type {}, and {}-bit color.", bmp.width(), bmp.height(), bmp.dib.compression, bmp.dib.bits_per_pixel).into())
    } else {
        Ok(bmp)
    }
}
enum Toolstate {
    None,
    Loaded(DecodedBMP, PathBuf),
    Error(Box<dyn Error>),
}


pub struct ToolData {
    preview_texture: Option<(TextureHandle, TextureHandle)>,
    preview_text: String,
    color: Color32,
    background_color: Color32,
    palette_2: bool,
    palette: Option<[Color32; 8]>,
    recalculate_textures: bool,
    last_operation: Option<RichText>,
}
pub struct Tool {
    state: Toolstate,
    data: ToolData,
    dock: egui_dock::DockState<Panel>,
    id_generator: u64,
}
fn load_font(state: &mut Toolstate, data: &mut ToolData) {
    data.preview_texture = None;
    let Some(bmp_path) = rfd::FileDialog::new().add_filter("BMP Image", &["bmp"]).set_title("Select a 4-Bit BMP to make your font from").pick_file() else { *state = Toolstate::Error("No file selected.".into()); return };
    match read_bmp(bmp_path.clone()) {
        Ok(img) => *state = Toolstate::Loaded(img, bmp_path),
        Err(e) => *state = Toolstate::Error(e)
    };
}
impl ToolData {
    fn nothing(&mut self, ui: &mut eframe::egui::Ui) -> Option<Toolstate> {
        let mut state = None;

        eframe::egui::Modal::new("banan".into()).show(ui.ctx(), |ui|{
            ui.vertical_centered(|ui|{

                ui.heading("Astronaut font converter");
                ui.label("Made by vikrinox, 2026");
                

                let s: Option<eframe::egui::Response> = ui.data(|r| r.get_temp("buttonguy".into()));
                let s = s.map(|i| ui.style().interact(&i).clone()).unwrap_or(ui.style().noninteractive().clone());


                let res = egui::Frame::new().stroke(s.bg_stroke).outer_margin(5.0-s.bg_stroke.width).inner_margin(10.0).corner_radius(s.corner_radius).fill(s.bg_fill).show(ui, |ui|{
                    
                    ui.set_width(120.0);
                    ui.add(egui::Image::new(eframe::egui::include_image!("../folder_icon.png")).max_size(Vec2::new(20.0, 20.0)).tint(s.fg_stroke.color));
                    ui.add(egui::Label::new(egui::RichText::new("Load Source BMP").color(s.fg_stroke.color)).selectable(false));
                }).response.interact(Sense::click());
               
                if res.clicked() {
                    load_font(state.get_or_insert(Toolstate::None), self);
                }

                ui.data_mut(|w| w.insert_temp("buttonguy".into(), res));

                ui.small(
                    "
                    This tool is made to create fonts for the Astronaut stage 2 mod on DSi consoles. 
                    It works by converting a 4-bit BMP sized 1024x8 into ascii character cells that are 7x8 pixels. 
                    Fonts may use 4 colors, with colors 0..=3 denoting the main palette, 
                    and colors 4..=7 denoting a optional second palette.
                    "
                );
            });
        });
        state
    }
    fn error(&mut self, ui: &mut eframe::egui::Ui, err: &Box<dyn Error>) -> Option<Toolstate> {
        eframe::egui::Modal::new("banan2".into()).show(ui.ctx(), |ui| {
            let error_text = format!("{}", err);
            ui.vertical_centered(|ui| {
                ui.add_space((ui.available_height()/2.)-50.0);

                ui.heading("Error:");
                ui.label(error_text);

                ui.button("Okay").clicked().then_some(Toolstate::None)
            }).inner
        }).inner
    }
}
pub fn load_gui_fonts(ctx: &egui::Context) {
    let (_a,_b,c) = system_fonts::find_for_system_locale(system_fonts::FontStyle::Sans);
    let mut definitions = egui::FontDefinitions::default();
    for font in c {
        let font_data = match font.source {
            system_fonts::FoundFontSource::Path(path_buf) => {
                let Ok(read_font) = std::fs::read(path_buf) else {continue;};
                egui::FontData::from_owned(read_font)
            },
            system_fonts::FoundFontSource::Bytes(bytes) => {
                egui::FontData::from_owned(bytes.to_vec())
            },
        };
        definitions.font_data.insert(font.key.clone(), Arc::new(font_data));
        definitions.families.entry(egui::FontFamily::Proportional).and_modify(|f| f.push(font.key.clone()));
    }
    ctx.set_fonts(definitions);

}
impl Tool {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);
        load_gui_fonts(&cc.egui_ctx);
        
        let state = Toolstate::None;
        
        let mut dock = egui_dock::DockState::new(vec![Panel::new(PreviewPanel, 0)]);
        
        dock.split(
            NodePath::MAIN_ROOT, 
            egui_dock::Split::Below, 
            0.8, 
            egui_dock::Node::Leaf(LeafNode::new(vec![Panel::new(PreviewControlsPanel, 1)]))
        );

        dock.split(
            NodePath::MAIN_ROOT, 
            egui_dock::Split::Left, 
            0.3, 
            egui_dock::Node::Leaf(LeafNode::new(vec![Panel::new(ColorPalettePanel, 2)]))
        );
        
        let data =  {
            let preview_texture = None;
            let preview_text = "The quick brown fox jumped over the lazy dog".to_string();
            let color = Color32::WHITE;
            let background_color = Color32::GRAY;
            let palette_2 = false;
            let palette: Option<[Color32; 8]> = None;    
            let recalculate_textures = false;
            let last_operation = None;
            ToolData { preview_texture, preview_text, color, background_color, palette_2, palette, recalculate_textures, last_operation }
        };
        let id_generator = 10;
        Tool {
            id_generator,
            state,
            data,
            dock,
        }
    }
}

pub struct TabViewer<'a> {
    data: &'a mut ToolData,
    state: &'a mut Toolstate,
}
impl<'a> egui_dock::TabViewer for TabViewer<'a> {
    type Tab = Panel;

    fn id(&mut self, tab: &mut Self::Tab) -> egui::Id {
        egui::Id::new(tab.id).with("dock tab")
    }

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        match &mut tab.ui {
            PanelType::Always(ui) => ui.name().into(),
            PanelType::Loaded(ui) => ui.name().into(),
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        match &mut tab.ui {
            PanelType::Always(always_ui) => always_ui.ui(ui, &mut self.data),
            PanelType::Loaded(loaded_ui) => match self.state {
                Toolstate::Loaded(image, bmp_path) => loaded_ui.ui(ui, &mut self.data, image, bmp_path),
                _ => ui.centered_and_justified(|ui| {ui.label("No Font Loaded.");}).inner,
            },
        }
    }
}
impl eframe::App for Tool {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::top("toolbar").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.style_mut().visuals.button_frame = false;
                ui.menu_button("File", |ui| {
                    if ui.button("Open Font BMP").clicked() {
                        load_font(&mut self.state, &mut self.data);
                    }

                    if let (Toolstate::Loaded(image, bmp_path), Some(color_palette)) = (&self.state, &self.data.palette) {
                        if ui.button("Export font.bin").clicked() {
                            if let Some(mut font) = build_tools::convert_font(image) {
                                font.truncate(2048);
                                font.insert(0, 0);
                                font.insert(0, 0);
                                font.extend(color_palette
                                .iter()
                                .map(|i| {
                                    let [b, g, r, _] = i.to_array();
                                    let r = ((r >> 3) as u16) << 0;
                                    let g = ((g >> 3) as u16) << 5;
                                    let b = ((b >> 3) as u16) << 10;
                                    (r | g | b).to_le_bytes()
                                }).flatten());
                                let d = rfd::FileDialog::new()
                                .add_filter("Astronaut font Binary", &["bin"])
                                .set_title("Select Location to export font.bin")
                                .set_file_name("font.bin")
                                .save_file();
                                if let Some(a) = d {
                                    self.data.last_operation = match fs::write(&a, font) {
                                        Ok(()) => Some(RichText::new(format!("Font exported to {:?}", &a)).color(Color32::GREEN)),
                                        Err(e) => Some(RichText::new(format!("Error: {}", e)).color(Color32::RED)),
                                    };
                                } 
                            } else {
                                //state = Some(Toolstate::Error(format!("An error occured while converting the font...").into()));
                            }
                        }
                    } else {
                        ui.add_enabled_ui(false, |ui| ui.button("Export Font"));
                    }
                });
                ui.menu_button("Panels", |ui| {
                    self.id_generator += 1; // i know this is dogshit, but it works tm
                    if ui.button("Color Palette").clicked() {
                        self.dock.push_to_first_leaf(Panel::new(ColorPalettePanel, self.id_generator));
                    }
                    if ui.button("Preview").clicked() {
                        self.dock.push_to_first_leaf(Panel::new(PreviewPanel, self.id_generator));
                    }
                    if ui.button("Preview Controls").clicked() {
                        self.dock.push_to_first_leaf(Panel::new(PreviewControlsPanel, self.id_generator));
                    }
                });
            });
        });
        eframe::egui::Panel::bottom("Status Bar").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Astronaut Font Builder");
                ui.label(env!("CARGO_PKG_VERSION"));
                ui.separator();
                if let Some(operation) = &self.data.last_operation {
                    ui.label(operation.clone());
                }
            });
        });
        let mut tab_viewer = TabViewer { data: &mut self.data, state: &mut self.state};
        
        eframe::egui::CentralPanel::default().frame(egui::Frame::NONE).show(ui, |ui| {
            if self.dock.iter_all_nodes().next().is_none() {
                ui.centered_and_justified(|ui| ui.label("No panels open."));
            } else {
                egui_dock::DockArea::new(&mut self.dock).show_inside(ui, &mut tab_viewer);
            }
        });

        let new_state = match &self.state {
            Toolstate::None => {
                self.data.nothing(ui)
            }
            Toolstate::Error(err) => {
                self.data.error(ui, err)
            }
            _ => None
        };
        if let Some(new_state) = new_state {
            self.state = new_state;
        }
       
    }
}
fn main() {
    let mut options = NativeOptions::default();
    options.viewport = options.viewport.with_maximized(true);
    eframe::run_native(
        "Simple Font Converter", 
        options, 
        Box::new(|cc| Ok(Box::new(Tool::new(cc))))
    ).expect("Failed to launch GUI for font converter");
}

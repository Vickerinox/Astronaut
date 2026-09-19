// SPDX-FileCopyrightText: 2026 Viktor Karlsson <viktor@koda.re>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::{error::Error, fs, path::PathBuf};

use build_tools::DecodedBMP;
use eframe::{
    NativeOptions, egui::{self, Color32, Pos2, Rect, Sense, TextureHandle, TextureOptions, Vec2}, emath::RectTransform,
};
use egui_dock::{LeafNode, NodePath};

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
pub struct Panel {
    id: u64,
    ui: PanelType,
}
pub enum PanelType {
    Always(Box<dyn AlwaysUi>),
    Loaded(Box<dyn LoadedUi>),
}

pub trait AlwaysUi: PanelUi {
    fn ui(&mut self, ui: &mut egui::Ui, data: &mut ToolData);
}
pub trait LoadedUi: PanelUi {
    fn ui(&mut self, ui: &mut egui::Ui, data: &mut ToolData, image: &DecodedBMP, bmp_path: &PathBuf);
}
pub trait PanelUi {
    fn name(&mut self) -> &str;
}


pub struct PreviewPanel;
pub struct ColorPalettePanel;
pub struct PreviewControlsPanel;


pub trait AsPanel {
    fn as_panel(self) -> PanelType;
}
impl Panel {
    pub fn new(panel: impl AsPanel, id: u64) -> Panel {
        Panel { id, ui: panel.as_panel() }
    }
}

impl<T: AlwaysUi + 'static> AsPanel for T {
    fn as_panel(self) -> PanelType {
        PanelType::Always(Box::new(self))
    }
}
impl AsPanel for PreviewPanel {
    fn as_panel(self) -> PanelType {
        PanelType::Loaded(Box::new(self))
    }
}

impl LoadedUi for PreviewPanel {
    fn ui(&mut self, ui: &mut egui::Ui, data: &mut ToolData, image: &DecodedBMP, bmp_path: &PathBuf) {
        let ToolData { preview_texture, preview_text, color, background_color, palette_2, palette , recalculate_textures} = data;
        ui.heading("Preview");
        let color_get_fn = || {
            let colors = image.palette_table();
            let color_map_fn = |i: u8| {
                    if i & 3 == 0 {
                        return Color32::TRANSPARENT;
                    }
                    let [r,g,b,_] = colors.get(i as usize).cloned().unwrap_or_default();
                    Color32::from_rgba_premultiplied(r, g, b, 255)
                };
            let mut palette = [Color32::WHITE; 8];
            for (i, color) in palette.iter_mut().enumerate() {
                *color = color_map_fn(i as u8);
            }
            palette
        };
        let color_palette = palette.get_or_insert_with(color_get_fn);
        let (default, alternative) = preview_texture.get_or_insert_with(|| {
            *recalculate_textures = false;
            let (texture, texture2) = {
                let pixel_split_fn = |i: &u8| {
                    [(i&0xF0) >> 4,i&0xF]
                };
                let cloned_palette = color_palette.clone();
                let color_map_fn = move |i: u8| -> Color32 {
                    cloned_palette.get(i as usize).copied().unwrap_or(Color32::WHITE)
                };
                let bitmap: Vec<_> = image
                        .bitmap().iter()
                        .map(pixel_split_fn).flatten().map(color_map_fn)
                        .collect();

                let bitmap2: Vec<_> = image
                        .bitmap().iter()
                        .map(pixel_split_fn).flatten().map(|i| i+4).map(color_map_fn)
                        .collect();
                (eframe::egui::ColorImage::new([1024, 8], bitmap),
                eframe::egui::ColorImage::new([1024, 8], bitmap2))
            };
            (
            ui.load_texture("font", texture, TextureOptions::NEAREST),    
            ui.load_texture("font_alt", texture2, TextureOptions::NEAREST)
            )
        });
        
        
        eframe::egui::Frame::new().fill(*background_color).inner_margin(8.0).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                let style = ui.spacing_mut();
                style.item_spacing = Vec2::new(0., 0.);
                for char in preview_text.chars() {
                    if char as u32 <= 0x80 {
                        let rect = RectTransform::from_to(Rect::from_min_size(Pos2::new(0.0, 1.0), Vec2::new(1.0, -1.0)), Rect::from_min_size(Pos2::ZERO, Vec2::new(1024., 8.))).inverse();

                        let char_size = Vec2::new(7.0, 8.0);
                        let char_rect = rect.transform_rect(Rect::from_min_size(Pos2::new((7 * char as u32) as f32, 0.), Vec2::new(7.0, 8.0)));

                        let (mut a,_b) = ui.allocate_exact_size((char_size-Vec2::new(1.0, 0.0))*2., Sense::empty());
                        a.extend_with_x(a.max.x+2.0);
                        let texture = if *palette_2 {
                            alternative.id()
                        } else {
                            default.id()
                        };
                        ui.painter().image(texture, a, char_rect, *color);
                    }
                }
            });
        });

       
        ui.label("Preview text:");
        ui.text_edit_multiline(preview_text);
    
        
        if *recalculate_textures {
            *preview_texture = None;
        }
    }
}
impl PanelUi for PreviewPanel {
    fn name(&mut self) -> &str {
        "Preview"
    }
}
impl PanelUi for ColorPalettePanel {
    fn name(&mut self) -> &str {
        "Color Palette"
    }
}
impl PanelUi for PreviewControlsPanel {
    fn name(&mut self) -> &str {
        "Preview Controls"
    }
}
impl AlwaysUi for PreviewControlsPanel {
    fn ui(&mut self, ui: &mut egui::Ui, data: &mut ToolData) {
        ui.horizontal(|ui| {
            ui.label("Text Color: ");
            ui.color_edit_button_srgba(&mut data.color);
            ui.label("Background color:");
            ui.color_edit_button_srgba(&mut data.background_color);
            ui.label("Use alternative palette:");
            ui.checkbox(&mut data.palette_2, "");
        });
    }
}
impl AlwaysUi for ColorPalettePanel {
    fn ui(&mut self, ui: &mut egui::Ui, data: &mut ToolData) {
        let Some(color_palette) = &mut data.palette else { ui.centered_and_justified(|ui| ui.label("No Palette available")); return; };
        ui.heading("Color Palette:");
        ui.group(|ui| {
            ui.label("Palette 1:");
            ui.horizontal_wrapped(|ui| {
                
                for color in color_palette[..4].iter_mut() {
                    if ui.color_edit_button_srgba(color).changed() {
                        data.recalculate_textures = true;
                    }
                }
            }); 
        });
        ui.group(|ui| {
            ui.label("Palette 2:");
            ui.horizontal_wrapped(|ui| {
                for color in color_palette[4..].iter_mut() {
                    if ui.color_edit_button_srgba(color).changed() {
                        data.recalculate_textures = true;
                    }
                }
            }); 
        });
    }
}

pub struct ToolData {
    preview_texture: Option<(TextureHandle, TextureHandle)>,
    preview_text: String,
    color: Color32,
    background_color: Color32,
    palette_2: bool,
    palette: Option<[Color32; 8]>,
    recalculate_textures: bool,
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
    fn preview(&mut self, ui: &mut eframe::egui::Ui, image: &DecodedBMP, bmp_path: &PathBuf) -> Option<Toolstate> {
        let mut state = None;
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
impl Tool {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);
        
        let state = Toolstate::None;
        
        let mut dock = egui_dock::DockState::new(vec![Panel::new(PreviewPanel, 0)]);
        
        dock.split(
            NodePath::MAIN_ROOT, 
            egui_dock::Split::Below, 
            0.3, 
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
            ToolData { preview_texture, preview_text, color, background_color, palette_2, palette, recalculate_textures }
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
                ui.menu_button("File", |ui| {
                    if ui.button("Open Font BMP").clicked() {
                        load_font(&mut self.state, &mut self.data);
                    }

                    if let (Toolstate::Loaded(image, bmp_path), Some(color_palette)) = (&self.state, &self.data.palette) {
                        if ui.button("Export Font").clicked() {
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
                                let mut a = bmp_path.clone();
                                a.pop();
                                let a = a.join("font.bin");
                                match fs::write(&a, font) {
                                    Ok(()) => self.data.preview_text = format!("Font saved to {:?}", a),
                                    Err(e) => (), //state = Some(Toolstate::Error(format!("Failed to write font to path {:?}, {}", &a, e).into()))
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

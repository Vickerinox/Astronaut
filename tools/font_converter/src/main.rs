// SPDX-FileCopyrightText: 2026 Viktor Karlsson <viktor@koda.re>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::{error::Error, fs, path::PathBuf};

use build_tools::DecodedBMP;
use eframe::{
    NativeOptions, egui::{self, Color32, Pos2, Rect, Sense, TextureHandle, TextureOptions, Vec2}, emath::RectTransform,
};

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
}
pub struct Tool {
    state: Toolstate,
    data: ToolData,
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
                    self.preview_texture = None;
                    let Some(bmp_path) = rfd::FileDialog::new().add_filter("BMP Image", &["bmp"]).set_title("Select a 4-Bit BMP to make your font from").pick_file() else { state = Some(Toolstate::Error("No file selected.".into())); return };
                    match read_bmp(bmp_path.clone()) {
                            Ok(img) => state = Some(Toolstate::Loaded(img, bmp_path)),
                            Err(e) => state = Some(Toolstate::Error(e))
                        };
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
        let Self { preview_texture, preview_text, color, background_color, palette_2, palette } = self;
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
        ui.horizontal(|ui| {
            ui.label("Preview text:");
            ui.text_edit_singleline(preview_text);
        });
        ui.horizontal(|ui| {
            ui.label("Text Color: ");
            ui.color_edit_button_srgba(color);
            ui.label("Use alternative palette:");
            ui.checkbox(palette_2, "");
            ui.label("Background color:");
            ui.color_edit_button_srgba(background_color)
        });
        ui.heading("Color Palette:");
        let mut recalculate_textures = false;
        ui.horizontal(|ui| {
            for color in color_palette.iter_mut() {
                if ui.color_edit_button_srgba(color).changed() {
                    recalculate_textures = true;
                }
            }
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
        if ui.button("Convert to font").clicked() {
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
                    //0xffffu16.to_le_bytes()
                }).flatten());
                let mut a = bmp_path.clone();
                a.pop();
                let a = a.join("font.bin");
                match fs::write(&a, font) {
                    Ok(()) => *preview_text = format!("Font saved to {:?}", a),
                    Err(e) => state = Some(Toolstate::Error(format!("Failed to write font to path {:?}, {}", &a, e).into()))
                }
            } else {
                state = Some(Toolstate::Error(format!("An error occured while converting the font...").into()));
            }
        }
        if recalculate_textures {
            *preview_texture = None;
        }
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
        let preview_texture = None;
        let preview_text = "The quick brown fox jumped over the lazy dog".to_string();
        let color = Color32::WHITE;
        let background_color = Color32::GRAY;
        let palette_2 = false;
        let palette: Option<[Color32; 8]> = None;
        let tool = Tool {
            state,
            data: ToolData { preview_texture, preview_text, color, background_color, palette_2, palette },
        };
        tool
    }
}
impl eframe::App for Tool {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, frame: &mut eframe::Frame) {
        eframe::egui::CentralPanel::default().show(ui, |ui| {
            let new_state = match &self.state {
                Toolstate::None => {
                    self.data.nothing(ui)
                }
                Toolstate::Loaded(image, bmp_path) => {
                    self.data.preview(ui, image, bmp_path)
                }
                Toolstate::Error(err) => {
                   self.data.error(ui, err)
                }
            };
            if let Some(new_state) = new_state {
                self.state = new_state;
            }
        });
    }
}
fn main() {
    eframe::run_native(
        "Simple Font Converter", 
        NativeOptions::default(), 
        Box::new(|cc| Ok(Box::new(Tool::new(cc))))
    ).expect("Failed to launch GUI for font converter");
}

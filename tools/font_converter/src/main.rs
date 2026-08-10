// SPDX-FileCopyrightText: 2026 Viktor Karlsson <viktor@koda.re>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::{error::Error, fs, path::PathBuf};

use build_tools::DecodedBMP;
use eframe::{
    NativeOptions,
    egui::{Color32, Pos2, Rect, Sense, TextureOptions, Vec2},
    emath::RectTransform,
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
fn main() {
    let mut state = Toolstate::None;
    let mut preview_texture = None;
    let mut preview_text = "The quick brown fox jumped over the lazy dog".to_string();
    let mut color = Color32::WHITE;
    let mut background_color = Color32::GRAY;
    let mut palette_2 = false;
    let mut palette: Option<[Color32; 8]> = None;
    eframe::run_ui_native("Simple Font Converter", NativeOptions::default(), move |ui, _frame| {
        eframe::egui::CentralPanel::default().show(ui, |ui| {
            match &state {
                Toolstate::None => {
                    ui.vertical_centered(|ui| {
                    ui.add_space((ui.available_height()/2.)-200.0);

                    ui.heading("Astronaut font converter:");
                    if ui.button("Load Source BMP").clicked() {
                        preview_texture = None;
                        let Some(bmp_path) = rfd::FileDialog::new().add_filter("BMP Image", &["bmp"]).set_title("Select a 4-Bit BMP to make your font from").pick_file() else { state = Toolstate::Error("No file selected.".into()); return };
                        match read_bmp(bmp_path.clone()) {
                                Ok(img) => state = Toolstate::Loaded(img, bmp_path),
                                Err(e) => state = Toolstate::Error(e)
                            };
                    }

                    ui.label("
                        Made by vikrinox, 2026 \n\n
                        This tool is made to create fonts for the Astronaut stage 2 mod on DSi consoles. 
                        It works by converting a 4-bit BMP sized 1024x8 into ascii character cells that are 7x8 pixels. 
                        Fonts may use 4 colors, with colors 0..=3 denoting the main palette, and colors 4..=7 denoting a optional second palette.
                    ");
                });
                }
                Toolstate::Loaded(image, bmp_path) => {
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
                        ui.load_texture("font", texture, TextureOptions::NEAREST),    ui.load_texture("font_alt", texture2, TextureOptions::NEAREST)
                        )
                    });
                    ui.horizontal(|ui| {
                        ui.label("Preview text:");
                        ui.text_edit_singleline(&mut preview_text);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Text Color: ");
                        ui.color_edit_button_srgba(&mut color);
                        ui.label("Use alternative palette:");
                        ui.checkbox(&mut palette_2, "");
                        ui.label("Background color:");
                        ui.color_edit_button_srgba(&mut background_color)
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
                    eframe::egui::Frame::new().fill(background_color).inner_margin(8.0).show(ui, |ui| {
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
                                    let texture = if palette_2 {
                                        alternative.id()
                                    } else {
                                        default.id()
                                    };
                                    ui.painter().image(texture, a, char_rect, color);
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
                                Ok(()) => preview_text = format!("Font saved to {:?}", a),
                                Err(e) => state = Toolstate::Error(format!("Failed to write font to path {:?}, {}", &a, e).into())
                            }
                        } else {
                            state = Toolstate::Error(format!("An error occured while converting the font...").into());
                        }
                    }
                    if recalculate_textures {
                        preview_texture = None;
                    }
                }
                Toolstate::Error(err) => {
                    let error_text = format!("{}", err);
                    ui.vertical_centered(|ui| {
                        ui.add_space((ui.available_height()/2.)-50.0);

                        ui.heading("Error:");
                        ui.label(error_text);

                        if ui.button("Load Source BMP again").clicked() {
                            preview_texture = None;
                            let Some(bmp_path) = rfd::FileDialog::new().add_filter("BMP Image", &["bmp"]).set_title("Select a 4-Bit BMP to make your font from").pick_file() else { state = Toolstate::Error("No file selected.".into()); return };
                            match read_bmp(bmp_path.clone()) {
                                Ok(img) => state = Toolstate::Loaded(img, bmp_path),
                                Err(e) => state = Toolstate::Error(e)
                            };
                        }
                    });
                }
        }
        });
    }).expect("Failed to launch GUI for font converter");
}

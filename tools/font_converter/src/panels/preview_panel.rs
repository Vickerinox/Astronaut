// SPDX-FileCopyrightText: 2026 Viktor Karlsson <viktor@koda.re>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::path::PathBuf;

use build_tools::DecodedBMP;
use eframe::{egui::{self, Color32, Pos2, Rect, Sense, TextureOptions, Vec2}, emath::RectTransform};

use crate::{ToolData, panels::{LoadedUi, PanelUi}};

pub struct PreviewPanel;

impl PanelUi for PreviewPanel {
    fn name(&mut self) -> &str {
        "Preview"
    }
}

impl LoadedUi for PreviewPanel {
    fn ui(&mut self, ui: &mut egui::Ui, data: &mut ToolData, image: &DecodedBMP, _bmp_path: &PathBuf) {
        let ToolData { preview_texture, preview_text, color, background_color, palette_2, palette , recalculate_textures, ..} = data;
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
                    if char == '\n' {
                        ui.end_row();
                        continue;
                    } 
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
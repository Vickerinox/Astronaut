use micro_imgui::{Rect, Vec2};
use reboot_lib::{VertexListHost, VertexListType, VideoHardwareHandle};

pub use crate::{DSMicroGuiBackend, Input};

pub struct VideoTextPass<'a>(&'a mut VideoHardwareHandle, micro_imgui::Rect);

impl<'a> VideoTextPass<'a> {
    pub fn new(hardware: &'a mut VideoHardwareHandle, available_space: micro_imgui::Rect) -> Self {
        Self(hardware, available_space)
    }
    pub unsafe fn text_pass<R, F: FnOnce(&mut TextLayoutHandle) -> R>(self, closure: F) -> R {
        let Self(host, available_space) = self;
        unsafe {
            host.create_vertex_list(VertexListType::IndividualQuads, |h| {
                let mut host = TextLayoutHandle {
                    available_space,
                    cursor: available_space.min,
                    host: h.to_owned(),
                    used_space: Rect::from_two_pos(available_space.min, available_space.min),
                };
                h.set_texture((7 << 20) | (2 << 26) | (1 << 29) | 0x3000);
                closure(&mut host)
            })
        }
    }
}
pub struct TextLayoutHandle<'a> {
    cursor: Vec2,
    available_space: Rect,
    used_space: Rect,
    host: VertexListHost<'a>,
}
impl<'a> TextLayoutHandle<'a> {
    pub fn used_space(&self) -> Rect {
        self.used_space
    }
    pub fn set_color(&mut self, color: u32) {
        self.host.set_vertex_color(color);
    }
    pub fn layout_str(&mut self, str: &str, size: u8) {
        for byte in str.as_bytes() {
            if !byte.is_ascii() {
                continue;
            }
            self.layout_char(*byte, size);
        }
    }
    pub fn set_position(&mut self, x: u8, y: u8) {
        self.cursor = Vec2::new(x as i16, y as i16);
    }
    pub fn next_line(&mut self) {
        self.cursor.x = self.available_space.min.x;
        self.cursor.y += 8;
    }
    pub fn layout_char(&mut self, ascii_value: u8, size: u8) {
        let y_size = size as i16;

        const CHAR_WIDTH: i16 = 7 << 4; //(i.e, 1*7 texels)
        let index = CHAR_WIDTH * ascii_value as i16;
        let movement = 6;
        let movement = (movement * y_size) >> 3;
        self.cursor.x += movement;

        if self.available_space.max.x < self.cursor.x {
            self.cursor.x = self.available_space.min.x + movement;
            self.cursor.y += y_size as i16;
        }
        self.used_space = self
            .used_space
            .include_point(Vec2::new(self.cursor.x + 1, self.cursor.y + y_size));
        let x = (self.cursor.x as i16 + 1) << 4;
        let y = (self.cursor.y as i16) << 4;
        unsafe {
            self.host
                .vertex_set_texture_coordinate(index + CHAR_WIDTH, 0x0);
            self.host.add_vertex_double(x, y, 0);
            self.host.vertex_set_texture_coordinate(index, 0x00);
            self.host
                .add_vertex_relative_raw(0b1111111111 - ((7 * size as u32) << (4 - 3)) + 1);
            self.host.vertex_set_texture_coordinate(index, 0x80);
            self.host
                .add_vertex_relative_raw(((size as u32) << 4) << 10);
            self.host
                .vertex_set_texture_coordinate(index + CHAR_WIDTH, 0x80);
            self.host
                .add_vertex_relative_raw((7 * size as u32) << (4 - 3));
        }
    }
}

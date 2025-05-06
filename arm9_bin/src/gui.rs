use core::num::NonZeroU16;

use reboot_lib::{Buttons, VertexListHost, VertexListType, VideoHardwareHandle};
use micro_imgui::{LayerId, Rect, Vec2};

pub struct DSMicroGuiBackend {
    input: Inputs,
    video: reboot_lib::VideoHardwareHandle,
    layer: u16,
}
impl DSMicroGuiBackend {
    pub fn new(video: reboot_lib::VideoHardwareHandle) -> Self {
        Self { input: Inputs {
            buttons_now: Buttons::empty(),
            buttons_then: Buttons::empty(),
            last_touch_coord: Vec2::ZERO,
            other_last_touch_coord: Vec2::ZERO,
        }, video, layer: 4 }
    }
    fn advance_layer(&mut self) -> LayerId {
        unsafe {
            let layer = LayerId(NonZeroU16::new_unchecked(self.layer));
            reboot_lib::VIDEO_HARDWARE
            .geometry_commands
            .translate_matrix(0, 0, 1<<3);
            self.layer += 1;
            layer
        }
    }
}
pub struct Input(pub Buttons);
impl From<Buttons> for Input {
    fn from(value: Buttons) -> Self {
        Self(value)
    }
}
impl micro_imgui::InputEvent for Input {
    const POINTER_DOWN: Self = Self(Buttons::PEN_DOWN);

    const POINTER_PRESS: Self = Self(Buttons::PEN_DOWN);

    const FOCUSED_PRESS: Self = Self(Buttons::BUTTON_A);

    const FOCUS_LEFT: Self = Self(Buttons::DIRECTION_LEFT);

    const FOCUS_RIGHT: Self = Self(Buttons::DIRECTION_RIGHT);

    const FOCUS_UP: Self = Self(Buttons::DIRECTION_UP);

    const FOCUS_DOWN: Self = Self(Buttons::DIRECTION_DOWN);

    const FOCUS_NEXT: Self = Self(Buttons::DIRECTION_RIGHT);

    const FOCUS_PREVIOUS: Self = Self(Buttons::DIRECTION_LEFT);
}


impl micro_imgui::Backend for DSMicroGuiBackend {
    type InputQuery = Input;

    fn gather_inputs(&mut self) -> bool {
        let buttons = unsafe { crate::read_controller() };
        self.input.update(buttons, Vec2::ZERO)
    }
    fn screen_rect(&self) -> Rect {
        Rect::from_min_size(micro_imgui::Vec2::ZERO, micro_imgui::Vec2::new(256, 192))
    }

    fn start_frame(&mut self) {
        unsafe {
            use reboot_lib::VIDEO_HARDWARE;
            self.video.init_matricies();
            VIDEO_HARDWARE
                .geometry_commands
                .select_matrix_stack(reboot_lib::MatrixMode::POSITION);
            VIDEO_HARDWARE
                .geometry_commands
                .scale_matrix(0x1000, -0x1555, -0x1000);
            VIDEO_HARDWARE
                .geometry_commands
                .scale_matrix(0x2000, 0x2000, 0x2000);
    
            VIDEO_HARDWARE
                .geometry_commands
                .translate_matrix(-0x80 * 0x10, -0x60 * 0x10, 100);
        }
    }

    fn end_frame(&mut self) {
        unsafe { self.video.next_frame() };
    }

    fn draw_shape(&mut self, shape: micro_imgui::Shape, regression: Option<LayerId>) -> Rect {

        
        
        let translation = regression.map(|i| self.layer.wrapping_sub(i.0.get())<<3).unwrap_or(0);
        if translation != 0 {
            unsafe {
                reboot_lib::VIDEO_HARDWARE
                .geometry_commands
                .translate_matrix(0, 0, -(translation as i32));
            }
        }
        
        self.advance_layer();
        
        let space = match shape {
            micro_imgui::Shape::Rectangle { area, fill: color, rounding, outline_color, outline_size } => {
                
                let Rect {min: micro_imgui::Vec2 {x,y}, max: Vec2 { x: x2, y: y2 }} = area;
                let x = x << 4;
                let y = y << 4;
                let x2 = x2 << 4;
                let y2 = y2 << 4;
                let outline_size = outline_size << 4;
                unsafe {
                    self.video.create_vertex_list(reboot_lib::VertexListType::IndividualQuads, |f| {
                        f.vertex_set_texture_coordinate(234 << 4, 1 << 4);
                        f.set_vertex_color(outline_color.0 as u32);
                        f.add_vertex_double(x, y, 0);
                        f.add_vertex_double(x, y2, 0);
                        f.add_vertex_double(x2, y2, 0);
                        f.add_vertex_double(x2, y, 0);
                        f.set_vertex_color(color.0 as u32);
                        let x = x.wrapping_add_unsigned(outline_size);
                        let y = y.wrapping_add_unsigned(outline_size);
                        let x2 = x2.wrapping_sub_unsigned(outline_size);
                        let y2 = y2.wrapping_sub_unsigned(outline_size);
                        f.add_vertex_double(x, y, 1);
                        f.add_vertex_double(x, y2, 1);
                        f.add_vertex_double(x2, y2, 1);
                        f.add_vertex_double(x2, y, 1);

                        
                    });
                }
                area
            },
            micro_imgui::Shape::Text { bounds, str, color, outline, size } => {
                let coord = bounds.min;
                let x = coord.x as u8;
                let y = coord.y as u8;
                unsafe {
                    VideoTextPass::new(&mut self.video, bounds).text_pass(|f| {
                        f.set_position(x, y);
                        f.set_color(color.0 as u32);
                        f.layout_str(&str, size);
                        f.used_space()
                    })
                }
            },
        };
        
        if translation != 0 {
            unsafe {
                reboot_lib::VIDEO_HARDWARE
                .geometry_commands
                .translate_matrix(0, 0, translation as i32 );
            } 
        }
        
        space
    }

    fn input_active(&self, pattern: Self::InputQuery) -> bool {
        self.input.keys_down(pattern.0)
    }

    fn input_pressed(&self, pattern: Self::InputQuery) -> bool {
        self.input.keys_pressed(pattern.0)
    }

    fn input_released(&self, pattern: Self::InputQuery) -> bool {
        self.input.keys_released(pattern.0)
    }

    fn last_known_pointer_location(&self) -> Vec2 {
        self.input.last_touch_coord
    }

    fn second_last_known_pointer_location(&self) -> Vec2 {
        self.input.last_touch_coord
    }
    
    fn reserve_layer(&mut self) -> LayerId {
        self.advance_layer()
    }
}


pub struct Inputs {
    buttons_now: Buttons,
    buttons_then: Buttons,
    last_touch_coord: Vec2,
    other_last_touch_coord: Vec2,
}
impl Inputs {
    pub fn keys_pressed(&self, buttons: Buttons) -> bool {
        (self.buttons_now & !self.buttons_then).contains(buttons)
    }
    pub fn keys_down(&self, buttons: Buttons) -> bool {
        self.buttons_now.contains(buttons)
    }
    pub fn keys_released(&self, buttons: Buttons) -> bool {
        (self.buttons_then & !self.buttons_now).contains(buttons)
    }
    pub fn last_known_touch_point(&self) -> Vec2 {
        self.last_touch_coord
    }
    pub fn update(&mut self, buttons: Buttons, touch_coord: Vec2) -> bool {
        self.buttons_then = self.buttons_now;
        self.buttons_now = buttons;
        if buttons.contains(Buttons::PEN_DOWN) {
            self.other_last_touch_coord = self.last_touch_coord;
            self.last_touch_coord = touch_coord;
        }
        (self.buttons_now != self.buttons_then) || (self.other_last_touch_coord != self.last_touch_coord)
    }
}



pub struct VideoTextPass<'a>(&'a mut VideoHardwareHandle, micro_imgui::Rect);

impl<'a> VideoTextPass<'a> {
    pub fn new(hardware: &'a mut VideoHardwareHandle, available_space: micro_imgui::Rect) -> Self {
        Self(hardware, available_space)
    }
    pub unsafe fn text_pass<R, F: FnOnce(&mut TextLayoutHandle) -> R>(self, closure: F) -> R {
        let Self(host, available_space) = self;
        host.create_vertex_list(VertexListType::IndividualQuads, |h| {
            let mut host = TextLayoutHandle {
                available_space,
                cursor: available_space.min,
                host: unsafe {
                     h.to_owned()
                },
                used_space: Rect::from_two_pos(available_space.min, available_space.min),
            };
            closure(&mut host)
        })
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
        let movement = match ascii_value {
            b'j' => 5,
            b'l' => 3,
            b'i' => 2,
            _ => 6,
        };
        let movement = (movement * y_size) >> 3;
        self.cursor.x += movement;
        
        if self.available_space.max.x < self.cursor.x {
            self.cursor.x = self.available_space.min.x + movement;
            self.cursor.y += y_size as i16;
        }
        self.used_space = self.used_space.include_point(Vec2::new(self.cursor.x-1, self.cursor.y+y_size));
        let x = (self.cursor.x as i16 + 1) << 4;
        let y = (self.cursor.y as i16) << 4;
        unsafe {
            self.host
                .vertex_set_texture_coordinate(index + CHAR_WIDTH, 0x0);
            self.host.add_vertex_double(x, y, 0);
            self.host.vertex_set_texture_coordinate(index, 0x00);
            self.host.add_vertex_relative_raw(0b1111111111 - ((7 * size as u32 )<<(4-3)) + 1);
            self.host.vertex_set_texture_coordinate(index, 0x80);
            self.host
                .add_vertex_relative_raw(((size as u32) << 4) << 10);
            self.host
                .vertex_set_texture_coordinate(index + CHAR_WIDTH, 0x80);
            self.host.add_vertex_relative_raw((7 * size as u32 )<<(4-3));
        }
    }
}
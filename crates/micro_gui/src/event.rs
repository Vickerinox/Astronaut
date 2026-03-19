use crate::primitives::Vec2;

#[derive(Default)]
pub struct Events {
    buttons_now: Buttons,
    buttons_then: Buttons,
    last_touch_coord: Vec2,
    other_last_touch_coord: Vec2,
}
impl Events {
    pub fn keys_pressed(&self, buttons: Buttons) -> bool {
        (self.buttons_now & !self.buttons_then).contains(buttons)
    }
    pub fn keys_released(&self, buttons: Buttons) -> bool {
        (self.buttons_then & !self.buttons_now).contains(buttons)
    }
    pub fn pen_down(&self) -> bool {
        self.buttons_now.contains(Buttons::Pen)
    }
    pub fn last_known_touch_point(&self) -> Vec2 {
        self.last_touch_coord
    }
    pub fn update(&mut self, buttons: Buttons, touch_coord: Vec2) {
        self.buttons_then = self.buttons_now;
        self.buttons_now = buttons;
        if buttons.contains(Buttons::Pen) {
            self.other_last_touch_coord = self.last_touch_coord;
            self.last_touch_coord = touch_coord;
        }
    }
}

bitflags::bitflags! {
    #[derive(Default, Clone, Copy)]
    pub struct Buttons: u16 {
        const A = (1<<0);
        const B = (1<<1);
        const Select = (1<<2);
        const Start = (1<<3);
        const Left = (1<<5);
        const Right = (1<<4);
        const Up = (1<<6);
        const Down = (1<<7);
        const R = (1<<8);
        const L = (1<<9);
        const X = (1<<10);
        const Y = (1<<11);
        const Pen = (1<<12);
        const Hinge = (1<<13);
    }
}

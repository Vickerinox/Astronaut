use alloc::borrow::Cow;

use crate::{primitives::Sizing, ui::{AutoAdd, Ui}, Backend, Color, Response, Sense, Shape, Vec2};

pub struct Button<'a> {
    text: Cow<'a, str>,
    size: Sizing,
}
impl<'a> Button<'a> {
    pub fn new(text: Cow<'a, str>, size: Sizing) -> Self {
        Self { text, size }
    }
}
impl<'t> AutoAdd for Button<'t> {
    fn ui<'a, 'b, B: Backend>(self, ui: &mut Ui<'a, 'b, B>) -> Response {
        let Self { text, size } = self;
        let prep_size = match size {
            Sizing::Automatic => Vec2::new(0, 8),
            Sizing::Cropped(vec2) => vec2.max(Vec2::new(0, 8)),
            Sizing::Padded(vec2) => vec2.max(Vec2::new(0, 8)),
        };
        let bounds = ui.prepare_complication(prep_size).0.translate(Vec2::unit(2)).include_point(ui.clip_rect().max);
        let box_shaper = ui.reserve_shape();
        let rect = ui.draw(Shape::Text { bounds, str: text, color: Color::new(200, 200, 200), outline: Color::new(0, 0, 0), size: 8 });
        let resp = ui.allocate_size(rect.scale_uniform(2).size(), Sense::clickable());

        let (outline_color, fill) = if resp.stats.intersects(Sense::PRESSED) {
            (Color::new(0, 0, 0), Color::new(32, 32, 32))
        } else if resp.stats.intersects(Sense::FOCUSED | Sense::HOVERED) {
            (Color::new(200, 200, 200), Color::new(100, 100, 100))
        } else {
            (Color::new(0, 0, 0), Color::new(100, 100, 100))
        };

        ui.draw_under(Shape::Rectangle { area: resp.rect, fill, rounding: 1, outline_color, outline_size: 1 }, box_shaper);
        resp
    }
}
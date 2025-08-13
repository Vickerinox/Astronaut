use alloc::borrow::Cow;

use crate::{
    primitives::Sizing,
    ui::{AutoAdd, Ui},
    Backend, Color, Response, Sense, Shape, Vec2,
};

pub struct Label<'a> {
    text: Cow<'a, str>,
    size: u8,
}
impl<'a> Label<'a> {
    pub fn new(text: impl Into<Cow<'a, str>>, size: u8) -> Self {
        Self {
            text: text.into(),
            size,
        }
    }
}
impl<'t> AutoAdd for Label<'t> {
    fn ui<'a, 'b, B: Backend>(self, ui: &mut Ui<'a, 'b, B>) -> Response {
        let Self { text, size } = self;
        let prep_size = Vec2::new(0, 8);

        let bounds = ui.prepare_complication(prep_size);
        let bounds = bounds.0.include_point(ui.clip_rect().max);
        let rect = ui.draw(Shape::Text {
            bounds,
            str: text,
            color: Color::new(200, 200, 200),
            outline: Color::new(0, 0, 0),
            size,
        });
        let resp = ui.allocate_size(rect.size(), Sense::hovered());
        resp
    }
}

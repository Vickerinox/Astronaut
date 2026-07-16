// SPDX-FileCopyrightText: 2026 Viktor Karlsson <viktor@koda.re>
// SPDX-License-Identifier: MIT

use crate::{
    ui::{AutoAdd, Ui},
    Backend, Color, Rect, Response, Sense, Shape, Vec2,
};

pub struct Checkbox<'a> {
    text: &'a str,
    option: &'a mut bool,
}
impl<'a> Checkbox<'a> {
    pub fn new(option: &'a mut bool, text: &'a str) -> Self {
        Self { text, option }
    }
}
impl<'t> AutoAdd for Checkbox<'t> {
    fn ui<'a, 'b, B: Backend>(self, ui: &mut Ui<'a, 'b, B>) -> Response {
        let Self { text, option } = self;
        let prep_size = Vec2::new(0, 8);
        let bounds = ui
            .prepare_complication(prep_size)
            .0
            .translate(Vec2::new(11, 1))
            .include_point(ui.clip_rect().max);
        let rect = ui.draw(Shape::Text {
            bounds,
            str: &text,
            color: ui.style().text_color,
            size: 8,
        });
        let wanted_size = rect.size();
        let alloc_size = wanted_size + Vec2::new(11, 2);
        let resp = ui.allocate_size(alloc_size, Sense::clickable());

        let colorset = ui.style_for(&resp).clone();

        let checkbox =
            Rect::from_min_size(resp.rect.top_left(), Vec2::unit(8)).translate(Vec2::unit(1));
        ui.draw(Shape::Rectangle {
            area: checkbox,
            fill: colorset.frame_fill,
            rounding: 1,
            outline_color: colorset.frame_outline,
            outline_size: 1,
        });
        if *option {
            ui.draw(Shape::Rectangle {
                area: checkbox.scale_uniform(-2),
                fill: Color::new(200, 200, 200),
                rounding: 1,
                outline_color: colorset.frame_outline,
                outline_size: 1,
            });
        }
        if resp.clicked() {
            *option = !*option;
        }
        resp
    }
}

// SPDX-FileCopyrightText: 2026 Viktor Karlsson <viktor@koda.re>
// SPDX-License-Identifier: MIT

use crate::{
    primitives::Sizing,
    ui::{AutoAdd, Ui},
    Backend, Color, Response, Sense, Shape, Vec2,
};

pub struct Button<'a> {
    text: &'a str,
    size: Sizing,
    text_color: Color,
}
impl<'a> Button<'a> {
    pub fn new(text: &'a str, size: Sizing, text_color: Color) -> Self {
        Self {
            text,
            size,
            text_color,
        }
    }
}
impl<'t> AutoAdd for Button<'t> {
    fn ui<'a, 'b, B: Backend>(self, ui: &mut Ui<'a, 'b, B>) -> Response {
        let Self {
            text,
            size,
            text_color,
        } = self;
        let prep_size = match size {
            Sizing::Automatic => Vec2::new(0, 8),
            Sizing::Cropped(vec2) => vec2.max(Vec2::new(0, 8)),
            Sizing::Padded(vec2) => vec2.max(Vec2::new(0, 8)),
        };
        let bounds = ui
            .prepare_complication(prep_size)
            .0
            .translate(Vec2::unit(3))
            .include_point(ui.clip_rect().max);
        let box_shaper = ui.reserve_shape();
        let rect = ui.draw(Shape::Text {
            bounds,
            str: &text,
            color: text_color,
            size: 8,
        });
        let wanted_size = rect.scale_uniform(3).size();
        let alloc_size = match size {
            Sizing::Automatic => wanted_size,
            Sizing::Cropped(_vec2) => prep_size.max(wanted_size),
            Sizing::Padded(_vec2) => prep_size.max(wanted_size),
        };
        let resp = ui.allocate_size(alloc_size, Sense::clickable());

        let colorset = ui.style_for(&resp);

        ui.draw_under(
            Shape::Rectangle {
                area: resp.rect.scale_uniform(-1),
                fill: colorset.frame_fill,
                rounding: 1,
                outline_color: colorset.frame_outline,
                outline_size: 1,
            },
            box_shaper,
        );
        resp
    }
}

use crate::{
    event::{Buttons, Events},
    primitives::{Id, Rect, Vec2},
    response::{Response, ResponseStats},
    ui::Ui,
};
use alloc::vec::Vec;
pub struct Ctx {
    id_generator: Id,
    input_events: Events,
    pressed_response: Option<Id>,
    hovered_response: Option<Id>,
    focused_response: Option<Id>,
    released_response: Option<Id>,
}
pub struct Frame<'a> {
    ctx: &'a mut Ctx,
    pressed_response: Option<Id>,
    hovered_response: Option<Id>,
    focused_response: Option<Id>,
    released_response: Option<Id>,
}
impl<'a> Frame<'a> {
    pub fn interact(&mut self, rect: Rect, clip_rect: Rect, id: Id) -> Response {
        let interact_rect = rect.intersect(clip_rect);
        let focused = self.ctx.focused_response == Some(id);
        let clicked = focused && self.ctx.input_events.keys_pressed(Buttons::A);
        let released = focused && self.ctx.input_events.keys_released(Buttons::A);
        if focused {
            self.focused_response = Some(id)
        }
        if clicked {
            self.pressed_response = Some(id)
        }
        if released {
            self.released_response = Some(id)
        }
        let mut stats = ResponseStats::empty();
        if Some(id) == self.ctx.focused_response {
            stats |= ResponseStats::FOCUSED;
        }
        if Some(id) == self.ctx.pressed_response {
            stats |= ResponseStats::CLICKED;
        }
        if Some(id) == self.ctx.released_response {
            stats |= ResponseStats::RELEASED;
        }
        if Some(id) == self.ctx.hovered_response {
            stats |= ResponseStats::HOVERED;
        }
        Response {
            id,
            rect,
            interact_rect,
            drag_delta: Vec2::ZERO,
            stats,
        }
    }
    pub fn central_panel<R, F: FnOnce(&mut Ui) -> R>(&mut self, f: F) -> R {
        let mut ui = Ui::new(self, Id::START);
        f(&mut ui)
    }
}
impl Ctx {
    pub fn new() -> Self {
        Self {
            input_events: Events::default(),
            pressed_response: None,
            hovered_response: None,
            focused_response: None,
            released_response: None,
            id_generator: Id::START,
        }
    }
    pub fn process_frame<R, F: FnOnce(&mut Frame) -> R>(
        &mut self,
        new_input: Buttons,
        pen: Vec2,
        f: F,
    ) -> R {
        self.input_events.update(new_input, pen);
        let mut frame = Frame {
            ctx: self,
            pressed_response: None,
            hovered_response: None,
            focused_response: None,
            released_response: None,
        };
        f(&mut frame)
    }

    pub fn end_frame(&mut self) {}
}

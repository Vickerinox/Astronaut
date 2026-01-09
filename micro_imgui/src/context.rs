use crate::{
    primitives::{Backend, Id, InputEvent, Rect, Vec2},
    response::{Response, Sense},
    ui::Ui,
};

pub struct Ctx<B> {
    pub(crate) backend: B,
    touchdown_pos: Option<Vec2>,
    pressed_response: Option<Id>,
    hovered_response: Option<Id>,
    focused_response: Option<Id>,
    released_response: Option<Id>,
    pub(crate) wants_repaint: bool,
}
pub struct Frame<'a, B: Backend> {
    ctx: &'a mut Ctx<B>,
    availble_ground_space: Rect,
    pressed_response: Option<Id>,
    hovered_response: Option<Id>,
    focused_response: Option<Id>,
    released_response: Option<Id>,

    prev_focus: Option<Id>,
    next_focus: Option<Id>,
}

impl<'a, B: Backend> core::ops::Deref for Frame<'a, B> {
    type Target = B;

    fn deref(&self) -> &Self::Target {
        &self.ctx.backend
    }
}
impl<'a, B: Backend> core::ops::DerefMut for Frame<'a, B> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.ctx.backend
    }
}

impl<'a, B: Backend> Frame<'a, B> {
    pub(crate) fn id_statistics(&self, id: Id) -> Sense {
        let mut stats = Sense::empty();
        if Some(id) == self.ctx.focused_response {
            stats |= Sense::FOCUSED;
        }
        if Some(id) == self.ctx.pressed_response {
            stats |= Sense::PRESSED;
        }
        if Some(id) == self.ctx.released_response {
            stats |= Sense::RELEASED;
        }
        if Some(id) == self.ctx.hovered_response {
            stats |= Sense::HOVERED;
        }
        stats
    }
    pub fn request_repaint(&mut self) {
        self.ctx.wants_repaint = true;
    }
    pub fn interact(&mut self, rect: Rect, clip_rect: Rect, id: Id, sense: Sense) -> Response {
        let Self {
            ctx,
            pressed_response,
            focused_response,
            released_response,
            prev_focus,
            next_focus,
            ..
        } = self;

        let interact_rect = rect.intersect(clip_rect);
        let focused = ctx.focused_response == Some(id);
        let hovered = rect.contains(ctx.backend.last_known_pointer_location())
            && ctx.backend.input_active(B::InputQuery::POINTER_DOWN);

        let pressed = (focused && ctx.backend.input_active(B::InputQuery::FOCUSED_PRESS))
            || (hovered && ctx.backend.input_active(B::InputQuery::POINTER_PRESS));
        
        let released = (focused && ctx.backend.input_released(B::InputQuery::FOCUSED_PRESS))
            || (ctx.pressed_response == Some(id)
                && ctx.backend.input_released(B::InputQuery::POINTER_PRESS));

        if focused {
            *focused_response = Some(id)
        } else {
            if sense.contains(Sense::FOCUSED) {
                if ctx.focused_response.is_none() {
                    next_focus.get_or_insert(id);
                    *prev_focus = Some(id)
                } else {
                    match focused_response.is_some() {
                        true => {
                            next_focus.get_or_insert(id);
                        }
                        false => *prev_focus = Some(id),
                    }
                }
            }
        }
        if pressed {
            *pressed_response = Some(id)
        }
        if released {
            *released_response = Some(id)
        }
        if hovered {}
        let stats = self.id_statistics(id).intersection(sense);
        Response {
            id,
            rect,
            interact_rect,
            drag_delta: Vec2::ZERO,
            stats,
        }
    }
    pub fn central_panel<R, F: FnOnce(&mut Ui<B>) -> R>(&mut self, f: F) -> R {
        let rect = self.availble_ground_space;
        let mut ui = Ui::new(self, Id::START, rect.scale_uniform(-4));
        /*
        ui.draw(crate::Shape::Rectangle {
            area: rect,
            fill: crate::Color::new(0, 0, 0),
            rounding: 0,
            outline_color: crate::Color::new(0, 0, 0),
            outline_size: 0,
        });
        */
        f(&mut ui)
    }
    pub fn window<R, F: FnOnce(&mut Ui<B>) -> R>(&mut self, rect: Rect, f: F) -> R {
        let mut ui = Ui::new(self, Id::from_layer(2), rect.scale_uniform(-4));
        ui.draw(crate::Shape::Rectangle {
            area: rect,
            fill: crate::Color::new(60, 60, 60),
            rounding: 0,
            outline_color: crate::Color::new(0, 0, 0),
            outline_size: 1,
        });
        f(&mut ui)
    }
}
impl<B> Ctx<B> {
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            pressed_response: None,
            hovered_response: None,
            focused_response: None,
            released_response: None,
            wants_repaint: false,
            touchdown_pos: None,
        }
    }
}
impl<B: Backend> Ctx<B> {
    pub fn process_frame<R, T, F: FnMut(Frame<B>, &mut T) -> R>(
        &mut self,
        mut f: F,
        t: &mut T,
    ) -> R {
        let frame = self.start_frame();
        let ret = f(frame, t);
        ret
    }
    pub fn start_frame(&mut self) -> Frame<'_, B> {
        self.backend.start_frame();
        let availble_ground_space = self.backend.screen_rect();
        if self.backend.input_pressed(B::InputQuery::POINTER_DOWN) {
            self.touchdown_pos = Some(self.backend.last_known_pointer_location());
        }
        Frame {
            ctx: self,
            pressed_response: None,
            hovered_response: None,
            focused_response: None,
            released_response: None,
            prev_focus: None,
            next_focus: None,
            availble_ground_space,
        }
    }
    pub fn end_frame(&mut self) {
        self.backend.end_frame();
        if self.backend.input_released(B::InputQuery::POINTER_DOWN) {
            self.touchdown_pos = None;
        }
    }
}
impl<'a, B: Backend> Drop for Frame<'a, B> {
    fn drop(&mut self) {
        let Self {
            ctx,
            pressed_response,
            hovered_response,
            mut focused_response,
            released_response,
            prev_focus,
            next_focus,
            ..
        } = self;
        if !ctx.backend.input_active(B::InputQuery::FOCUSED_PRESS) {
            if ctx.backend.input_pressed(B::InputQuery::FOCUS_NEXT) {
                focused_response = *next_focus;
            }
            if ctx.backend.input_pressed(B::InputQuery::FOCUS_PREVIOUS) {
                focused_response = *prev_focus;
            }
        }
        ctx.end_frame();
        if *pressed_response != ctx.pressed_response {
            ctx.pressed_response = *pressed_response;
            ctx.wants_repaint = true;
        }
        if *hovered_response != ctx.hovered_response {
            ctx.hovered_response = *hovered_response;
            ctx.wants_repaint = true;
        }
        if focused_response != ctx.focused_response {
            ctx.focused_response = focused_response;
            ctx.wants_repaint = true;
        }
        if *released_response != ctx.released_response {
            ctx.released_response = *released_response;
            ctx.wants_repaint = true;
        }
    }
}

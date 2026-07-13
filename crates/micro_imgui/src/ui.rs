use crate::{
    context::{ColorSet, Frame},
    primitives::{Id, Rect, Vec2},
    response::{self, Response, Sense},
    widgets::{button::Button, label::Label},
    Backend, Color, LayerId, Style,
};

pub struct Ui<'a, 'b: 'a, B: Backend> {
    ctx: &'a mut Frame<'b, B>,
    clip_rect: Rect,
    id: Id,
    layout: Layout,
}
impl<'a, 'b: 'a, B: Backend> core::ops::Deref for Ui<'a, 'b, B> {
    type Target = Frame<'b, B>;
    fn deref(&self) -> &Self::Target {
        &self.ctx
    }
}
impl<'a, 'b: 'a, B: Backend> core::ops::DerefMut for Ui<'a, 'b, B> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.ctx
    }
}
impl<'a, 'b: 'a, B: Backend> Ui<'a, 'b, B> {
    pub fn new(ctx: &'a mut Frame<'b, B>, id: Id, clip_rect: Rect) -> Self {
        Self {
            ctx,
            clip_rect,
            id,
            layout: Layout::default(),
        }
    }

    pub fn set_focus(&mut self, response: &Response) {
        self.ctx.focus_on(Some(response.id));
    }
    pub fn clear_focus(&mut self) {
        self.ctx.focus_on(None);
    }
    pub fn button<'c>(&mut self, text: &str) -> response::Response {
        self.add(Button::new(
            text,
            crate::Sizing::Automatic,
            self.style().text_color,
        ))
    }
    pub fn backend(&self) -> &B {
        &self.ctx.backend
    }
    pub fn label<'c>(&mut self, text: &str) -> response::Response {
        self.add(Label::new(text, 8))
    }
    pub fn header<'c>(&mut self, text: &str) -> response::Response {
        self.add(Label::new(text, 16))
    }
    /// This is the libraries big hack for issues where the coupling between systems clash
    ///
    /// This function assumes two things:
    /// 1. the id of the widget you're about to prepare is the same one you'll be allocating after this
    /// 2. the id of the widget hasn't importantly changed in the last frame
    ///
    /// if you meet these conditions, you now have premature response that assumes you want EVERYTHING
    pub fn prepare_complication(&self, size: Vec2) -> (Rect, Sense) {
        let Layout(direction, _) = self.layout;
        let rect_min = match direction {
            Direction::TopDown => self.clip_rect.min,
            Direction::LeftRight => self.clip_rect.min,
        };
        let response = self.ctx.id_statistics(unsafe { self.id.current() });
        (Rect::from_two_pos(rect_min, rect_min + size), response)
    }

    pub fn horizontal<R>(&mut self, closure: impl FnOnce(&mut Ui<'a, 'b, B>) -> R) -> R {
        let old_clip_rect = self.clip_rect();
        let old_layout = self.layout.clone();
        self.layout = Layout(Direction::LeftRight, Align::Min);
        let ret = closure(self);
        self.layout = old_layout;
        self.clip_rect = old_clip_rect;
        self.add_space(16);
        ret
    }

    pub fn allocate_size(&mut self, size: Vec2, sense: Sense) -> Response {
        let Layout(direction, align) = self.layout;
        let (rsd_min, rsd_max) = match direction {
            Direction::TopDown => (Vec2::y(size.y), Vec2::ZERO),
            Direction::LeftRight => (Vec2::x(size.x), Vec2::ZERO),
        };
        let remaining_space = Rect {
            min: self.clip_rect.min + rsd_min,
            max: self.clip_rect.max + rsd_max,
        };

        let rect_min = match direction {
            Direction::TopDown => self.clip_rect.min,
            Direction::LeftRight => self.clip_rect.min,
        };
        let rect = Rect::from_two_pos(rect_min, rect_min + size);
        let rect = match (direction, align) {
            (Direction::TopDown, Align::Middle) => {
                rect.translate(Vec2::x((self.clip_rect.width() - size.x) >> 1))
            }
            (Direction::TopDown, Align::Max) => {
                rect.translate(Vec2::x(self.clip_rect.width() - size.x))
            }
            (Direction::LeftRight, Align::Middle) => {
                rect.translate(Vec2::y((self.clip_rect.height() - size.y) >> 1))
            }
            (Direction::LeftRight, Align::Max) => {
                rect.translate(Vec2::y(self.clip_rect.height() - size.y))
            }
            (Direction::TopDown, Align::Justified) => rect.set_width(self.clip_rect.width()),
            (Direction::LeftRight, Align::Justified) => rect.set_height(self.clip_rect.height()),
            _ => rect,
        };
        let id = self.id.next();
        let response = self.ctx.interact(rect, self.clip_rect, id, sense);
        self.clip_rect = remaining_space;
        return response;
    }

    #[inline]
    pub fn add(&mut self, widget: impl AutoAdd) -> Response {
        widget.ui(self)
    }

    pub fn add_space(&mut self, ammount: i16) {
        match self.layout.0 {
            Direction::TopDown => self.clip_rect.min.y += ammount,
            Direction::LeftRight => self.clip_rect.min.x += ammount,
        }
    }

    pub fn clip_rect(&self) -> Rect {
        self.clip_rect
    }

    pub fn draw(&mut self, shape: crate::primitives::Shape<B::Image>) -> Rect {
        self.ctx.draw_shape(shape, None)
    }

    pub fn draw_under(
        &mut self,
        shape: crate::primitives::Shape<B::Image>,
        layer: LayerId,
    ) -> Rect {
        self.ctx.draw_shape(shape, Some(layer))
    }

    pub fn reserve_shape(&mut self) -> LayerId {
        self.ctx.reserve_layer()
    }
}
pub trait AutoAdd {
    fn ui<'a, 'b, B: Backend>(self, ui: &mut Ui<'a, 'b, B>) -> Response;
}
#[derive(Default, Clone, Copy)]
pub struct Layout(Direction, Align);
#[allow(unused)]
#[derive(Default, Clone, Copy)]
pub enum Direction {
    #[default]
    TopDown,
    LeftRight,
}
#[allow(unused)]
#[derive(Default, Clone, Copy)]
pub enum Align {
    #[default]
    Min,
    Middle,
    Max,
    Justified,
}

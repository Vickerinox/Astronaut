use crate::{
    context::Frame,
    primitives::{Id, Rect, Vec2},
    response::{self, Response, Sense},
    widgets::{button::Button, label::Label},
    Backend, Color, LayerId,
};

pub struct Ui<'a, 'b: 'a, B: Backend> {
    ctx: &'a mut Frame<'b, B>,
    clip_rect: Rect,
    id: Id,
    layout: Layout,
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
    pub fn paint_shape(&mut self, shape: crate::primitives::Shape) {
        self.ctx.paint_shape(shape)
    }
    pub fn drag(&mut self) -> Option<Vec2> {
        self.ctx.drag()
    }
    pub fn cancel_refocus(&mut self) {
        self.ctx.cancel_refocus();
    }
    pub fn has_focus_anywhere(&mut self) -> bool {
        self.ctx.has_focus_anywhere()
    }
    pub fn button<'c>(&mut self, text: &str) -> response::Response {
        self.add(Button::new(
            text,
            crate::Sizing::Automatic,
            Color::new(200, 200, 200),
        ))
    }
    pub fn label<'c>(&mut self, text: &str) -> response::Response {
        self.add(Label::new(text, 8))
    }
    pub fn header<'c>(&mut self, text: &str) -> response::Response {
        self.add(Label::new(text, 16))
    }
    pub fn request_repaint(&mut self) {
        self.ctx.request_repaint();
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
            Direction::BottomUp => self.clip_rect.bottom_left() - Vec2::y(size.y),
            Direction::RightLeft => self.clip_rect.top_right() - Vec2::x(size.x),
        };
        let response = self.ctx.id_statistics(unsafe { self.id.current() });
        (Rect::from_two_pos(rect_min, rect_min + size), response)
    }
    pub fn input_down(&self, input: B::InputQuery) -> bool {
        self.ctx.input_active(input)
    }
    pub fn input_pressed(&self, input: B::InputQuery) -> bool {
        self.ctx.input_pressed(input)
    }
    pub fn input_released(&self, input: B::InputQuery) -> bool {
        self.ctx.input_released(input)
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
            Direction::BottomUp => (Vec2::ZERO, -Vec2::y(size.y)),
            Direction::RightLeft => (Vec2::ZERO, -Vec2::x(size.x)),
        };
        let remaining_space = Rect {
            min: self.clip_rect.min + rsd_min,
            max: self.clip_rect.max + rsd_max,
        };

        let rect_min = match direction {
            Direction::TopDown => self.clip_rect.min,
            Direction::LeftRight => self.clip_rect.min,
            Direction::BottomUp => self.clip_rect.bottom_left() - Vec2::y(size.y),
            Direction::RightLeft => self.clip_rect.top_right() - Vec2::x(size.x),
        };
        let rect = Rect::from_two_pos(rect_min, rect_min + size);
        let rect = match (direction, align) {
            (Direction::TopDown, Align::Middle) | (Direction::BottomUp, Align::Middle) => {
                rect.translate(Vec2::x((self.clip_rect.width() - size.x) >> 1))
            }
            (Direction::TopDown, Align::Max) | (Direction::BottomUp, Align::Max) => {
                rect.translate(Vec2::x(self.clip_rect.width() - size.x))
            }
            (Direction::LeftRight, Align::Middle) | (Direction::RightLeft, Align::Middle) => {
                rect.translate(Vec2::y((self.clip_rect.height() - size.y) >> 1))
            }
            (Direction::LeftRight, Align::Max) | (Direction::RightLeft, Align::Max) => {
                rect.translate(Vec2::y(self.clip_rect.height() - size.y))
            }
            (Direction::TopDown, Align::Justified) | (Direction::BottomUp, Align::Justified) => {
                rect.set_width(self.clip_rect.width())
            }
            (Direction::RightLeft, Align::Justified) | (Direction::LeftRight, Align::Justified) => {
                rect.set_height(self.clip_rect.height())
            }
            _ => rect,
        };
        let id = self.id.next();
        let response = self.ctx.interact(rect, self.clip_rect, id, sense);
        self.clip_rect = remaining_space;
        return response;
    }
    pub fn add(&mut self, widget: impl AutoAdd) -> Response {
        widget.ui(self)
    }
    pub fn add_space(&mut self, ammount: i16) {
        match self.layout.0 {
            Direction::RightLeft => self.clip_rect.max.x -= ammount,
            Direction::TopDown => self.clip_rect.min.y += ammount,
            Direction::LeftRight => self.clip_rect.min.x += ammount,
            Direction::BottomUp => self.clip_rect.max.y -= ammount,
        }
    }
    pub fn clip_rect(&self) -> Rect {
        self.clip_rect
    }
    pub fn draw(&mut self, shape: crate::primitives::Shape) -> Rect {
        self.ctx.draw_shape(shape, None)
    }
    pub fn draw_under(&mut self, shape: crate::primitives::Shape, layer: LayerId) -> Rect {
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
    BottomUp,
    RightLeft,
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

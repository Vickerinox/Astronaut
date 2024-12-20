use crate::{
    context::{Ctx, Frame},
    primitives::{Id, Rect, Vec2},
    response::Response,
};

pub struct Ui<'a, 'b: 'a> {
    ctx: &'a mut Frame<'b>,
    clip_rect: Rect,
    id: Id,
    layout: Layout,
}
impl<'a, 'b: 'a> Ui<'a, 'b> {
    pub fn new(ctx: &'a mut Frame<'b>, id: Id) -> Self {
        Self {
            ctx,
            clip_rect: Rect::SCREEN_RECT,
            id,
            layout: Layout::default(),
        }
    }
    pub fn allocate_size(&mut self, size: Vec2) -> Response {
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
        let rect = Rect::from_two_pos(rect_min, size);
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
        let response = self.ctx.interact(rect, self.clip_rect, id);
        self.clip_rect = remaining_space;
        return response;
    }
}
#[derive(Default, Clone, Copy)]
pub struct Layout(Direction, Align);
#[derive(Default, Clone, Copy)]
pub enum Direction {
    #[default]
    TopDown,
    LeftRight,
    BottomUp,
    RightLeft,
}
#[derive(Default, Clone, Copy)]
pub enum Align {
    #[default]
    Min,
    Middle,
    Max,
    Justified,
}

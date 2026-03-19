use crate::primitives::{Id, Rect, Vec2};

pub struct Response {
    pub id: Id,
    pub rect: Rect,
    pub interact_rect: Rect,
    pub drag_delta: Vec2,
    pub stats: ResponseStats,
}
bitflags::bitflags! {
    pub struct ResponseStats: u8 {
        const CLICKED = (1<<0);
        const HOVERED = (1<<1);
        const DRAGGED = (1<<2);
        const ENABLED = (1<<3);
        const FOCUSED = (1<<4);
        const RELEASED = (1<<5);

    }
}

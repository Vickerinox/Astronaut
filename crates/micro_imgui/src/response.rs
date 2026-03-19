use crate::primitives::{Id, Rect, Vec2};

pub struct Response {
    pub id: Id,
    pub rect: Rect,
    pub interact_rect: Rect,
    pub drag_delta: Vec2,
    pub stats: Sense,
}
bitflags::bitflags! {
    pub struct Sense: u8 {
        const PRESSED = (1<<0);
        const HOVERED = (1<<1);
        const DRAGGED = (1<<2);
        const ENABLED = (1<<3);
        const FOCUSED = (1<<4);
        const RELEASED = (1<<5);
    }
}
impl Response {
    pub fn clicked(&self) -> bool {
        self.stats.contains(Sense::RELEASED)
    }
    pub fn hovered(&self) -> bool {
        self.stats.contains(Sense::HOVERED)
    }
}
impl Sense {
    pub fn clickable() -> Self {
        Self::PRESSED | Self::HOVERED | Self::FOCUSED | Self::RELEASED
    }
    pub fn hovered() -> Self {
        Self::HOVERED
    }
}

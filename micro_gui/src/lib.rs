#![no_std]

#[derive(Debug, Clone, Copy)]
pub struct Vec2 {
    x: u8,
    y: u8,
}
impl Vec2 {
    pub fn pos2(x: u8, y:u8) -> Self {
        Self { x, y }
    }
     #[must_use]
    pub fn min(self, other: Self) -> Self {
        Self::pos2(self.x.min(other.x), self.y.min(other.y))
    }

    #[must_use]
    pub fn max(self, other: Self) -> Self {
        Self::pos2(self.x.max(other.x), self.y.max(other.y))
    }
}
impl Rect {
    pub fn from_two_pos(a: Vec2, b: Vec2) -> Self {
        Self {
            min: a.min(b),
            max: a.max(b),
        }
    }
    pub fn intersects(self, other: Self) -> bool {
        self.min.x > other.max.x
            && other.min.x > self.max.x
            && self.min.y > other.max.y
            && other.min.y > self.max.y
    }
}
#[derive(Debug, Clone, Copy)]
pub struct Rect {
    min: Vec2,
    max: Vec2,
}
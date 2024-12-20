use core::num::NonZeroU16;

#[derive(Clone, Copy, Hash, PartialEq)]
pub struct Id(NonZeroU16);

impl Id {
    pub const START: Self = Self(unsafe { NonZeroU16::new_unchecked(1) });

    pub fn next(&mut self) -> Self {
        unsafe {
            self.0 = self.0.unchecked_add(1);
        }
        self.clone()
    }
    pub fn child(&mut self) -> Self {
        unsafe {
            self.0 = self.0.unchecked_add(0x100);
        }
        return Self(unsafe { self.0.unchecked_add(0x1000) });
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Vec2 {
    pub x: i16,
    pub y: i16,
}
impl core::ops::Neg for Vec2 {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self {
            x: -self.x,
            y: -self.y,
        }
    }
}
impl core::ops::Add for Vec2 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x.saturating_add(rhs.x), self.y.saturating_add(rhs.y))
    }
}
impl core::ops::Sub for Vec2 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x.saturating_sub(rhs.x), self.y.saturating_sub(rhs.y))
    }
}
impl Vec2 {
    pub const ZERO: Self = Self::new(0, 0);
    pub const fn x(x: i16) -> Self {
        Self::new(x, 0)
    }
    pub const fn y(y: i16) -> Self {
        Self::new(0, y)
    }
    pub const fn new(x: i16, y: i16) -> Self {
        Self { x, y }
    }
    pub const fn unit(unit: i16) -> Self {
        Self::new(unit, unit)
    }
    #[must_use]
    pub fn min(self, other: Self) -> Self {
        Self::new(self.x.min(other.x), self.y.min(other.y))
    }

    #[must_use]
    pub fn max(self, other: Self) -> Self {
        Self::new(self.x.max(other.x), self.y.max(other.y))
    }
}
impl Rect {
    pub fn from_two_pos(a: Vec2, b: Vec2) -> Self {
        Self {
            min: a.min(b),
            max: a.max(b),
        }
    }
    pub fn from_min_size(a: Vec2, b: Vec2) -> Self {
        Self::from_two_pos(a, a + b)
    }
    pub fn intersects(self, other: Self) -> bool {
        self.min.x > other.max.x
            && other.min.x > self.max.x
            && self.min.y > other.max.y
            && other.min.y > self.max.y
    }
    pub fn intersect(self, other: Self) -> Self {
        Self {
            min: self.min.max(other.min),
            max: self.max.min(other.max),
        }
    }
    pub fn contains(&self, p: Vec2) -> bool {
        self.min.x <= p.x && p.x <= self.max.x && self.min.y <= p.y && p.y <= self.max.y
    }
    pub fn translate(self, offset: Vec2) -> Self {
        Self {
            min: self.min + offset,
            max: self.max + offset,
        }
    }

    pub fn left(&self) -> i16 {
        self.min.x
    }
    pub fn top(&self) -> i16 {
        self.min.y
    }
    pub fn right(&self) -> i16 {
        self.max.x
    }
    pub fn bottom(&self) -> i16 {
        self.max.y
    }
    pub fn top_left(&self) -> Vec2 {
        self.min
    }
    pub fn bottom_right(&self) -> Vec2 {
        self.max
    }
    pub fn top_right(&self) -> Vec2 {
        Vec2::new(self.max.x, self.min.y)
    }
    pub fn bottom_left(&self) -> Vec2 {
        Vec2::new(self.min.x, self.max.y)
    }
    pub fn height(&self) -> i16 {
        self.bottom() - self.top()
    }
    pub fn width(&self) -> i16 {
        self.right() - self.left()
    }
    pub fn set_height(mut self, height: i16) -> Self {
        self.max.y = self.min.y + height;
        self
    }
    pub fn set_width(mut self, width: i16) -> Self {
        self.max.x = self.min.x + width;
        self
    }
    pub fn scale_uniform(self, addition: i16) -> Self {
        Self::from_two_pos(
            self.min - Vec2::unit(addition),
            self.max + Vec2::unit(addition),
        )
    }
}
#[derive(Debug, Clone, Copy, Default)]
pub struct Rect {
    pub min: Vec2,
    pub max: Vec2,
}
impl Rect {
    pub const SCREEN_RECT: Self = Self {
        min: Vec2::new(0, 0),
        max: Vec2::new(255, 191),
    };
}

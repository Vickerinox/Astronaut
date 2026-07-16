// SPDX-FileCopyrightText: 2026 Viktor Karlsson <viktor@koda.re>
// SPDX-License-Identifier: MIT

use core::num::NonZeroU16;

pub struct LayerId(pub NonZeroU16);

#[derive(Clone, Copy, Hash, PartialEq)]
pub struct Id(NonZeroU16);

impl Id {
    pub const START: Self = Self(unsafe { NonZeroU16::new_unchecked(1) });
    pub const fn from_layer(layer: u8) -> Self {
        Self(unsafe { NonZeroU16::new_unchecked(1 + ((layer as u16) << 12)) })
    }
    pub fn next(&mut self) -> Self {
        unsafe {
            self.0 = self.0.unchecked_add(1);
        }
        self.clone()
    }
    pub unsafe fn current(&self) -> Self {
        Self(self.0.unchecked_add(1))
    }
    pub fn child(&mut self) -> Self {
        self.next();
        return Self(unsafe { self.0.unchecked_add(0x100) });
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Vec2 {
    pub x: i16,
    pub y: i16,
}
impl Vec2 {
    pub fn abs(self) -> Self {
        Self {
            x: self.x.abs(),
            y: self.y.abs(),
        }
    }
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
        Self::new(self.x.wrapping_add(rhs.x), self.y.wrapping_add(rhs.y))
    }
}
impl core::ops::Sub for Vec2 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x.wrapping_sub(rhs.x), self.y.wrapping_sub(rhs.y))
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
    pub fn from_center_size(center: Vec2, expansion: Vec2) -> Self {
        let expansion = expansion.abs();
        Self {
            min: center - expansion,
            max: center + expansion,
        }
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
    pub fn include_point(self, point: Vec2) -> Self {
        Self {
            min: self.min.min(point),
            max: self.max.max(point),
        }
    }
    pub fn size(self) -> Vec2 {
        self.max - self.min
    }
}
#[derive(Debug, Clone, Copy, Default)]
pub struct Rect {
    pub min: Vec2,
    pub max: Vec2,
}

pub trait Image {}
pub trait Backend {
    type InputQuery: InputEvent;
    type Image;
    /// The resolution of the screen at this moment
    fn screen_rect(&self) -> Rect;

    /// Update the underlying input events, set to true if this triggers a new frame
    fn gather_inputs(&mut self) -> bool;

    /// do work that needs to be done before a frame is started
    fn start_frame(&mut self);

    /// do work that needs to be done after a frame is finished
    fn end_frame(&mut self);

    /// Draw a shape to the current frame, and return the area which it occupied
    fn draw_shape(&mut self, shape: Shape<Self::Image>, layer: Option<LayerId>) -> Rect;

    fn reserve_layer(&mut self) -> LayerId;

    /// See if an input is currently held (keyboard key is down, button is pressed, etc.)
    fn input_down(&self, pattern: Self::InputQuery) -> bool;

    /// See if an input begun being held this frame
    fn input_pressed(&self, pattern: Self::InputQuery) -> bool;

    /// See if an input sopped being held this frame
    fn input_released(&self, pattern: Self::InputQuery) -> bool;

    /// Last known location of whatever pointer device is used
    fn last_known_pointer_location(&self) -> Vec2;
    fn second_last_known_pointer_location(&self) -> Vec2;
}
pub trait InputEvent {
    //if the pointer is down, for example on computers this is always the case. However on touch devices like phones it may not be.
    const POINTER_DOWN: Self;
    //if the pointer is pressing, equivalent to a mouse click on computer, or a tap on a phone
    const POINTER_PRESS: Self;
    //If a designated button is pressed. For example a focused element on computers can often be interacted with by pressing ENTER
    const FOCUSED_PRESS: Self;

    const FOCUS_LEFT: Self;
    const FOCUS_RIGHT: Self;
    const FOCUS_UP: Self;
    const FOCUS_DOWN: Self;

    const FOCUS_NEXT: Self;
    const FOCUS_PREVIOUS: Self;
}
#[derive(Clone, Copy, Default, PartialEq)]
pub struct Color(pub u16);
impl Color {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        let r = (r as u16 & 0b11111000) >> 3;
        let g = (g as u16 & 0b11111000) << 2;
        let b = (b as u16 & 0b11111000) << 7;
        Self(r | g | b | 0x8000)
    }
    pub const fn new_transparent(r: u8, g: u8, b: u8) -> Self {
        let r = (r as u16 & 0b11111000) >> 3;
        let g = (g as u16 & 0b11111000) << 2;
        let b = (b as u16 & 0b11111000) << 7;
        Self(r | g | b)
    }
}
pub enum Sizing {
    Automatic,
    Cropped(Vec2),
    Padded(Vec2),
}
pub enum Shape<'a, I> {
    Rectangle {
        area: Rect,
        fill: Color,
        rounding: u16,
        outline_color: Color,
        outline_size: u16,
    },
    Text {
        bounds: Rect,
        str: &'a str,
        color: Color,
        size: u8,
    },
    Image {
        bounds: Rect,
        image: I,
    },
}

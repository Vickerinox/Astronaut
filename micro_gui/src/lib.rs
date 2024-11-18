#![no_std]

pub struct Pos2 {
    x: u8,
    y: u8,
}
pub struct Vec2 {
    x: u8,
    y: u8,
}
pub struct DsRect {
    min: Pos2,
    max: Pos2,
}
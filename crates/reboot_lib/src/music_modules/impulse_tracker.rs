// SPDX-FileCopyrightText: 2026 Viktor Karlsson <viktor@koda.re>
// SPDX-License-Identifier: MIT

#[repr(C)]
pub struct ITHeader {
    signature: [u8; 4],
    name: [u8; 26],
    highlight: u16,

    order_number: u16,
    instrument_number: u16,
    sample_number: u16,
    pattern_number: u16,
    tracker_version: u16,
    format_version: u16,
    flags: u16,
    special: u16,

    global_volume: u8,
    mix_volume: u8,
    initial_speed: u8,
    initial_tempo: u8,
    panning_seperation: u8,
    pitch_wheel_depth: u8,
    message_length: u16,
    message: u16,
    offset: u16,
    reserved: u32,

    channel_panning: [u8; 64],
    channel_volume: [u8; 64],

    orders: *mut [ITOrder],
    samples: *mut [*mut ITSample],
    instruments: *mut [*mut ITInstrument],
    patterns: *mut [*mut ITPattern],
}
pub struct ITOrder {}
pub struct ITSample {}
pub struct ITInstrument {}
pub struct ITPattern {}

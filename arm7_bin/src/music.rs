use core::ops::{Add, Sub};

use common::bootstrap::BootStub;
use reboot_lib::ndma::Channel;
use reboot_lib::sound::{timer_from_freq, RepeatMode, SoundControl, SoundFormat, SOUND_HARDWARE};

static MUSIC_FRAME: &[u16] = &[
    0xF000 | 20,
    0x4000 | 20,
    0x8000 | 20,
    0xF000 | 15,
    0x4000 | 20,
    0x8000 | 15,
    0xF000 | 20,
    0x4000 | 15,
    0xF000 | 18,
    0x8000 | 20,
    0x8000 | 18,
    0xF000 | 15,
    0x4000 | 20,
    0x8000 | 15,
    0xF000 | 18,
    0x4000 | 15,
    0xF000 | 17,
    0x8000 | 18,
    0x8000 | 17,
    0xF000 | 13,
    0x4000 | 18,
    0x8000 | 13,
    0xF000 | 17,
    0x4000 | 13,
    0xF000 | 15,
    0x8000 | 17,
    0xF000 | 13,
    0x8000 | 15,
    0xF000 | 15,
    0x8000 | 13,
    0xF000 | 8,
    0x8000 | 15,
    0xF000 | 20,
    0x8000 | 8,
    0x8000 | 20,
    0xF000 | 15,
    0x4000 | 20,
    0x8000 | 15,
    0xF000 | 20,
    0x4000 | 15,
    0xF000 | 18,
    0x8000 | 20,
    0x8000 | 18,
    0xF000 | 15,
    0x4000 | 20,
    0x8000 | 15,
    0xF000 | 18,
    0x2000 | 15,
    0xF000 | 17,
    0x8000 | 18,
    0x8000 | 17,
    0xF000 | 13,
    0x4000 | 18,
    0x8000 | 13,
    0xF000 | 17,
    0x4000 | 13,
    0xF000 | 23,
    0x8000 | 17,
    0xF000 | 22,
    0x8000 | 23,
    0xF000 | 20,
    0x8000 | 22,
    0xF000 | 18,
    0x8000 | 20,
];

static MUSIC_FRAME_BASS: &[u16] = &[
    0xC000 | 8,
    0xC000 | 8,
    0x0000 | 8,
    0x0000 | 8,
    0xC000 | 8,
    0xC000 | 8,
    0x0000 | 8,
    0x0000 | 8,
    0xC000 | 8,
    0xC000 | 20,
    0x0000 | 8,
    0xC000 | 8,
    0x0000 | 8,
    0xC000 | 8,
    0xC000 | 8,
    0xC000 | 8,
    0x0000 | 8,
    0x0000 | 8,
    0xC000 | 8,
    0x0000 | 8,
    0xC000 | 8,
    0x0000 | 8,
    0xC000 | 6,
    0x0000 | 8,
    0xC000 | 8,
    0xC000 | 20,
    0x0000 | 8,
    0xC000 | 6,
    0xC000 | 8,
    0x0000 | 8,
    0xC000 | 3,
    0x0000 | 8,
];
static MUSIC_PITCHES: &[u16] = &[
    //OCTAVE 1
    reboot_lib::sound::timer_from_freq(4186 >> 4),
    reboot_lib::sound::timer_from_freq(4435 >> 4),
    reboot_lib::sound::timer_from_freq(4699 >> 4),
    reboot_lib::sound::timer_from_freq(4978 >> 4),
    reboot_lib::sound::timer_from_freq(5274 >> 4),
    reboot_lib::sound::timer_from_freq(5588 >> 4),
    reboot_lib::sound::timer_from_freq(5920 >> 4),
    reboot_lib::sound::timer_from_freq(6272 >> 4),
    reboot_lib::sound::timer_from_freq(6645 >> 4),
    reboot_lib::sound::timer_from_freq(7040 >> 4),
    reboot_lib::sound::timer_from_freq(7459 >> 4),
    reboot_lib::sound::timer_from_freq(7902 >> 4),
    //OCTAVE 1
    reboot_lib::sound::timer_from_freq(4186 >> 3),
    reboot_lib::sound::timer_from_freq(4435 >> 3),
    reboot_lib::sound::timer_from_freq(4699 >> 3),
    reboot_lib::sound::timer_from_freq(4978 >> 3),
    reboot_lib::sound::timer_from_freq(5274 >> 3),
    reboot_lib::sound::timer_from_freq(5588 >> 3),
    reboot_lib::sound::timer_from_freq(5920 >> 3),
    reboot_lib::sound::timer_from_freq(6272 >> 3),
    reboot_lib::sound::timer_from_freq(6645 >> 3),
    reboot_lib::sound::timer_from_freq(7040 >> 3),
    reboot_lib::sound::timer_from_freq(7459 >> 3),
    reboot_lib::sound::timer_from_freq(7902 >> 3),
    //OCTAVE 1
    reboot_lib::sound::timer_from_freq(4186 >> 2),
    reboot_lib::sound::timer_from_freq(4435 >> 2),
    reboot_lib::sound::timer_from_freq(4699 >> 2),
    reboot_lib::sound::timer_from_freq(4978 >> 2),
    reboot_lib::sound::timer_from_freq(5274 >> 2),
    reboot_lib::sound::timer_from_freq(5588 >> 2),
    reboot_lib::sound::timer_from_freq(5920 >> 2),
    reboot_lib::sound::timer_from_freq(6272 >> 2),
    reboot_lib::sound::timer_from_freq(6645 >> 2),
    reboot_lib::sound::timer_from_freq(7040 >> 2),
    reboot_lib::sound::timer_from_freq(7459 >> 2),
    reboot_lib::sound::timer_from_freq(7902 >> 2),
    //OCTAVE 1
    reboot_lib::sound::timer_from_freq(4186 >> 1),
    reboot_lib::sound::timer_from_freq(4435 >> 1),
    reboot_lib::sound::timer_from_freq(4699 >> 1),
    reboot_lib::sound::timer_from_freq(4978 >> 1),
    reboot_lib::sound::timer_from_freq(5274 >> 1),
    reboot_lib::sound::timer_from_freq(5588 >> 1),
    reboot_lib::sound::timer_from_freq(5920 >> 1),
    reboot_lib::sound::timer_from_freq(6272 >> 1),
    reboot_lib::sound::timer_from_freq(6645 >> 1),
    reboot_lib::sound::timer_from_freq(7040 >> 1),
    reboot_lib::sound::timer_from_freq(7459 >> 1),
    reboot_lib::sound::timer_from_freq(7902 >> 1),
    //OCTAVE 1
    reboot_lib::sound::timer_from_freq(4186),
    reboot_lib::sound::timer_from_freq(4435),
    reboot_lib::sound::timer_from_freq(4699),
    reboot_lib::sound::timer_from_freq(4978),
    reboot_lib::sound::timer_from_freq(5274),
    reboot_lib::sound::timer_from_freq(5588),
    reboot_lib::sound::timer_from_freq(5920),
    reboot_lib::sound::timer_from_freq(6272),
    reboot_lib::sound::timer_from_freq(6645),
    reboot_lib::sound::timer_from_freq(7040),
    reboot_lib::sound::timer_from_freq(7459),
    reboot_lib::sound::timer_from_freq(7902),
];

pub const fn amiga_to_nds_period(period: u16) -> u16 {
    0xFFFF - ((33513982 / 2) / (3549546 / period as u32)) as u16
}
use reboot_lib::music_modules::mods::*;

pub enum PitchModulation {
    SlideUp { ammount: u8 },
    SlideDown { ammount: u8 },
    Vibrato { command_value: u8, duration: u16 },
    Portamento { target: u16, rate: u8 },
    Arpeggio { value: u8, duration: u16 },
}
pub struct MODPlayData {
    ticks_per_row: u8,
    tick: u8,
    row: u8,
    frame: u8,
    last_sample_index: [u8; 4],
    target_pitch: [u16; 4],
    pitch_bend: [u16; 4],
    last_portamento_index: [u8; 4],
    current_song: *mut MODHeader,
}
impl MODPlayData {
    pub const fn defaults() -> Self {
        Self {
            ticks_per_row: 6,
            tick: 0,
            row: 0,
            frame: 0,
            pitch_bend: [0u16; 4],
            current_song: core::ptr::null_mut(),
            target_pitch: [0u16; 4],
            last_sample_index: [0u8; 4],
            last_portamento_index: [0u8; 4],
        }
    }
}
pub fn finetune_period(period: u16, finetune: u8) -> u16 {
    let ret = match finetune {
        15 => (period as u32 * 65535) / 65064, /* finetune -1, period factor 1.0072382087 */
        14 => (period as u32 * 65535) / 64596, /* finetune -2, period factor 1.01452880907 */
        13 => (period as u32 * 65535) / 64132, /* finetune -3, period factor 1.02187218032 */
        12 => (period as u32 * 65535) / 63671, /* finetune -4, period factor 1.02926870442 */
        11 => (period as u32 * 65535) / 63213, /* finetune -5, period factor 1.03671876611 */
        10 => (period as u32 * 65535) / 62760, /* finetune -6, period factor 1.04422275291 */
        9 => (period as u32 * 65535) / 62309,  /* finetune -7, period factor 1.05178105512 */
        8 => (period as u32 * 65535) / 61860,  /* finetune -8, period factor 1.05939406591 */
        7 => (period as u32 * 65535) / 68928,  /* finetune +7, period factor 0.95076821847 */
        6 => (period as u32 * 65535) / 68433,  /* finetune +6, period factor 0.95765007726 */
        5 => (period as u32 * 65535) / 67941,  /* finetune +5, period factor 0.96458174838 */
        4 => (period as u32 * 65535) / 67453,  /* finetune +4, period factor 0.97156359238 */
        3 => (period as u32 * 65535) / 66968,  /* finetune +3, period factor 0.97859597243 */
        2 => (period as u32 * 65535) / 66487,  /* finetune +2, period factor 0.98567925431 */
        1 => (period as u32 * 65535) / 66009,  /* finetune +1, period factor 0.99281380646 */
        _ => period as u32,
    } as u16;
    ret
}
pub static mut MODULE: MODPlayData = MODPlayData::defaults();
pub fn play_mod() {
    unsafe {
        let MODPlayData {
            ticks_per_row,
            tick,
            row,
            frame,
            current_song,
            pitch_bend: pitch_cache,
            target_pitch,
            last_sample_index,
            last_portamento_index,
        } = &mut MODULE;
        let Some(borrow) = current_song.as_mut() else {
            return;
        };
        let mut jump_to = None;

        if let Some(patterns) = borrow.patterns.as_mut() {
            if let Some(pattern) = patterns.get(borrow.frames[*frame as usize] as usize) {
                if *tick == 0 {
                    for (channel_id, note) in pattern[*row as usize].iter().enumerate() {
                        const CHANNEL_PANNINGS: [u8; 4] = [48, 80, 80, 48];
                        let mut period = note.period();
                        let sample = note.sample_index();

                        let command = note.command();

                        if command.kind == 0xF {
                            if command.value < 0x20 {
                                *ticks_per_row = command.value;
                            } else {
                                let tick_rate = command.value as u32 * 2 / 5;
                                let timer_counter = (33513982 / 64) / tick_rate as u32;
                                (0x4000100 as *mut u16)
                                    .write_volatile(0xFFFF - timer_counter as u16 + 1);
                            }
                        }
                        let channel = &reboot_lib::sound::SOUND_HARDWARE.channels[channel_id];
                        let mut control = channel.control.read().difference(SoundControl::START);

                        if sample > 0 {
                            if last_sample_index[channel_id] != sample - 1 {
                                channel.control.write(SoundControl::empty());
                            }
                            last_sample_index[channel_id] = sample - 1;
                        }

                        let Some(info) = &borrow
                            .sample_info
                            .get(last_sample_index[channel_id] as usize)
                        else {
                            continue;
                        };
                        control = if command.kind == 0xC {
                            control.with_volume((command.value << 1).saturating_sub(1) & 0x7f)
                        } else if period != 0 || sample != 0 {
                            control.with_volume(((info.volume << 1) - 1) & 0x7f)
                        } else {
                            control
                        };

                        if period != 0 {
                            let Some(adr) =
                                borrow.samples[last_sample_index[channel_id] as usize].as_mut()
                            else {
                                continue;
                            };

                            if command.kind == 0x3 {
                                if command.value != 0 {
                                    last_portamento_index[channel_id] = command.value
                                }
                            } else {
                                channel.control.write(SoundControl::empty())
                            }

                            period = finetune_period(period, info.finetune);

                            let sample_offset = if command.kind == 0x9 {
                                (command.value as u32) << 8
                            } else {
                                0
                            };

                            let len = (adr.len() as u32).saturating_sub(sample_offset) >> 2;
                            let adr = core::ptr::addr_of!(*adr) as *const u8 as u32;

                            if info.repeat_length <= 1 {
                                control = control.with_repeat_mode(RepeatMode::Oneshot);
                                channel.source.write(adr + sample_offset);
                                channel.length.write(len);
                                channel.loop_start.write(0);
                            } else {
                                let sample_offset =
                                    sample_offset.min((info.repeat_point as u32) << 1);

                                channel.source.write(adr + sample_offset);
                                control = control.with_repeat_mode(RepeatMode::Infinite);
                                channel.length.write((info.repeat_length >> 1) as u32);
                                channel.loop_start.write(
                                    (info.repeat_point >> 1)
                                        .saturating_sub((sample_offset >> 2) as u16),
                                );
                                //since the nds hardware doesn't have more than 4-byte granularity,
                                //there needs to be adjustment to loop frequencies that don't fall on that boundary.
                                //this tries to preserves pitch, but can destory timbre of single-wavecycle loops.
                                if info.repeat_length & 1 > 0 {
                                    period = {
                                        let supposed_length = info.repeat_length as u64;
                                        let actual_length = (info.repeat_length - 1) as u64;
                                        ((period as u64 * supposed_length * 0xFFFFFF)
                                            / (actual_length * 0xFFFFFF))
                                            as u16
                                    };
                                }
                            };
                            control |= SoundControl::empty()
                                .with_sound_format(SoundFormat::PCM8)
                                .with_panning(CHANNEL_PANNINGS[channel_id]);

                            target_pitch[channel_id] = period;
                            if !(command.kind == 0x3) {
                                pitch_cache[channel_id] = period;
                                channel.timer.write(amiga_to_nds_period(period));
                            }
                        }

                        if command.kind == 0xE {
                            match command.value & 0xF0 {
                                0xA0 => {
                                    let new_volume = control
                                        .volume()
                                        .saturating_add((command.value & 0xF) << 1)
                                        .min(127);
                                    control = control.with_volume(new_volume);
                                }
                                0xB0 => {
                                    let new_volume = control
                                        .volume()
                                        .saturating_sub((command.value & 0xF) << 1)
                                        .max(0);
                                    control = control.with_volume(new_volume);
                                }
                                0x10 => {
                                    let old_timer = pitch_cache[channel_id];
                                    let new_timer = old_timer - (command.value & 0xF) as u16;
                                    pitch_cache[channel_id] = new_timer;
                                    channel.timer.write(amiga_to_nds_period(new_timer));
                                }
                                0x20 => {
                                    let old_timer = pitch_cache[channel_id];
                                    let new_timer = old_timer + (command.value & 0xF) as u16;
                                    pitch_cache[channel_id] = new_timer;
                                    channel.timer.write(amiga_to_nds_period(new_timer));
                                }
                                _ => (),
                            }
                        }
                        if channel.control.read().contains(SoundControl::START) || period != 0 {
                            control |= SoundControl::START;
                        }
                        channel.control.write(control);
                    }
                } else {
                    for (channel_id, note) in pattern[*row as usize].iter().enumerate() {
                        let command = note.command();

                        let channel = &reboot_lib::sound::SOUND_HARDWARE.channels[channel_id];

                        match command.kind {
                            0xA => {
                                let increase = ((command.value & 0xF0) >> 4) as i8;
                                let decrease = (command.value & 0xF) as i8;
                                let change = (increase - decrease) * 2;
                                let current_control = channel.control.read();
                                let new_volume = current_control
                                    .volume()
                                    .saturating_add_signed(change)
                                    .max(0)
                                    .min(127);
                                let new_control = current_control.with_volume(new_volume);
                                channel.control.write(new_control);
                            }
                            0x3 => {
                                let value = last_portamento_index[channel_id];
                                let current_pitch = pitch_cache[channel_id];
                                let target_pitch = target_pitch[channel_id];
                                let new_timer = if current_pitch < target_pitch {
                                    current_pitch.saturating_add(value as u16).min(target_pitch)
                                } else {
                                    current_pitch.saturating_sub(value as u16).max(target_pitch)
                                };
                                pitch_cache[channel_id] = new_timer;
                                channel.timer.write(amiga_to_nds_period(new_timer));
                            }
                            0x1 => {
                                let old_timer = pitch_cache[channel_id];
                                let new_timer = old_timer - command.value as u16;
                                pitch_cache[channel_id] = new_timer;
                                channel.timer.write(amiga_to_nds_period(new_timer));
                            }
                            0x2 => {
                                let old_timer = pitch_cache[channel_id];
                                let new_timer = old_timer + command.value as u16;
                                pitch_cache[channel_id] = new_timer;
                                channel.timer.write(amiga_to_nds_period(new_timer));
                            }
                            0xB => {
                                jump_to = Some((command.value, 0));
                            }
                            0xD => {
                                jump_to = Some((*frame + 1, command.value));
                            }
                            _ => (),
                        }
                    }
                }
            }
        }

        *tick += 1;
        if *tick >= *ticks_per_row {
            *tick = 0;
            if let Some((jump_to, new_row)) = jump_to {
                *frame = jump_to;
                *row = new_row;
            } else {
                *row += 1;
            }
        }
        if *row >= 64 {
            *row = 0;
            *frame += 1;
        }
        if *frame >= borrow.song_length {
            *frame = 0;
        }
    }
}
pub fn set_mod(module: *mut MODHeader) {
    unsafe {
        reboot_lib::disable_interrupt(reboot_lib::ARM7Interrupt::Timer0);
        SOUND_HARDWARE.init();
        super::update_volume();
        MODULE = MODPlayData {
            current_song: module,
            ..MODPlayData::defaults()
        };
        (0x4000100 as *mut u32).write_volatile(0);
        (0x4000100 as *mut u32).write_volatile((0xFFFF - 10473) | 0x00C10000);
        reboot_lib::set_interrupt_function(reboot_lib::ARM7Interrupt::Timer0, play_mod as *mut _);
        reboot_lib::enable_interrupt(reboot_lib::ARM7Interrupt::Timer0);
    }
}
pub fn set_procedural() {
    unsafe {
        reboot_lib::disable_interrupt(reboot_lib::ARM7Interrupt::Timer0);
        SOUND_HARDWARE.init();
        super::update_volume();
        (0x4000100 as *mut u32).write_volatile(0);
        (0x4000100 as *mut u32).write_volatile((0xFFFF - 8800) | 0x00C10000);
        reboot_lib::set_interrupt_function(
            reboot_lib::ARM7Interrupt::Timer0,
            music_routine as *mut _,
        );
        reboot_lib::enable_interrupt(reboot_lib::ARM7Interrupt::Timer0);
    }
}
pub static mut FRAME_COUNTER: u32 = 0;
static mut MUSIC_COUNTER: u16 = 0;
pub fn music_routine() {
    unsafe fn play_melody_channel(
        channel: usize,
        volume_shift: u8,
        pan: u8,
        delay: u16,
        add: usize,
    ) {
        let channel = &reboot_lib::sound::SOUND_HARDWARE.channels[channel];
        let [note, volume] =
            MUSIC_FRAME[((MUSIC_COUNTER + 64 - delay) & 0x3F) as usize].to_le_bytes();
        channel.timer.write(MUSIC_PITCHES[note as usize + add]);
        let duty = ((MUSIC_COUNTER + 7 - delay) >> 3) as u32 % 7;
        let control_echo = SoundControl::START
            .with_repeat_mode(RepeatMode::Oneshot)
            .with_sound_format(SoundFormat::PSG)
            .with_panning(pan)
            .with_volume(volume >> volume_shift)
            | SoundControl::from_bits_retain(duty << 24);
        channel.control.write(control_echo);
    }
    unsafe {
        if FRAME_COUNTER % 7 == 0 {
            if MUSIC_COUNTER >= 128 {
                let channel = &reboot_lib::sound::SOUND_HARDWARE.channels[15];
                channel.timer.write(0xFFFF);
                let control = SoundControl::START
                    .with_repeat_mode(RepeatMode::Oneshot)
                    .with_sound_format(SoundFormat::PSG)
                    .with_panning(64)
                    .with_volume(20);
                channel.control.write(control);
            }
            if MUSIC_COUNTER >= 318 {
                let beat = MUSIC_COUNTER & 0xF;
                if beat == 0 || beat == 4 || beat == 7 || beat == 10 || beat == 12 {
                    let channel = &reboot_lib::sound::SOUND_HARDWARE.channels[1];
                    channel.timer.write(timer_from_freq(22050));
                    let adr = include_bytes!("./kick.raw");
                    channel.source.write(core::ptr::addr_of!(*adr) as u32 & !3);
                    channel.length.write((adr.len() as u32) >> 2);
                    let control = SoundControl::START
                        .with_repeat_mode(RepeatMode::Oneshot)
                        .with_sound_format(SoundFormat::PCM8)
                        .with_panning(40)
                        .with_volume(127);
                    channel.control.write(control);
                }
                if beat == 4 || beat == 12 || (MUSIC_COUNTER % 320) > 316 {
                    let channel = &reboot_lib::sound::SOUND_HARDWARE.channels[2];
                    channel.control.write(SoundControl::empty());
                    channel.timer.write(timer_from_freq(22050));
                    let adr = include_bytes!("./snare.raw");
                    channel.source.write(core::ptr::addr_of!(*adr) as u32 & !3);
                    channel.length.write((adr.len() as u32) >> 2);
                    let control = SoundControl::START
                        .with_repeat_mode(RepeatMode::Oneshot)
                        .with_sound_format(SoundFormat::PCM8)
                        .with_panning(80)
                        .with_volume(127);
                    channel.control.write(control);
                }
            }

            let add = if MUSIC_COUNTER >= 64 {
                let [note, volume] =
                    MUSIC_FRAME_BASS[(MUSIC_COUNTER & 0x1F) as usize].to_le_bytes();
                let channel = &reboot_lib::sound::SOUND_HARDWARE.channels[8];
                channel.timer.write(MUSIC_PITCHES[note as usize]);

                let control = SoundControl::START
                    .with_repeat_mode(RepeatMode::Oneshot)
                    .with_sound_format(SoundFormat::PSG)
                    .with_panning(64)
                    .with_volume(volume >> 2)
                    | SoundControl::from_bits_retain(3 << 24);
                channel.control.write(control);

                if (MUSIC_COUNTER / 64) % 4 == 3 {
                    24
                } else {
                    36
                }
            } else {
                24
            };
            play_melody_channel(9, 3, 64, 0, add);
            play_melody_channel(10, 5, 0, 3, add);
            play_melody_channel(11, 5, 127, 5, add);

            MUSIC_COUNTER += 1;
        } else {
            let ptr = &raw mut reboot_lib::sound::SOUND_HARDWARE.channels[8] as *mut u8;
            ptr.write_volatile(ptr.read().saturating_sub(1));
            let ptr = &raw mut reboot_lib::sound::SOUND_HARDWARE.channels[15] as *mut u8;
            let dec = if MUSIC_COUNTER % 16 != 7 { 10 } else { 1 };

            ptr.write_volatile(ptr.read().saturating_sub(dec));
        }

        FRAME_COUNTER += 1;
    };
}

#[repr(C)]
pub struct ImpulseHeader {
    sign: [u8; 4],
    name: [u8; 26],
    highlight: u16,
    order_len: u16,
    instrument_len: u16,
    sample_len: u16,
    pattern_len: u16,
    tracker_version_cwt: u16,
    format_version_cmwt: u16,
    flags: u16,
    special: u16,
    gv: u8,
    mv: u8,
    is: u8,
    it: u8,
    sp: u8,
    pwd: u8,
    msg_len: u16,
    msg_off: u32,
    reserved: u32,
    pannings: [u8; 64],
    volumes: [u8; 64],
    orders: *mut [u8],
    instruments: *mut ImpulseInstrument,
    samples: *mut ImpulseSample,
    patterns: *mut ImpulsePattern,
}

pub struct ImpulseModule {
    name: [u8; 26],
    flags: u16,
    special: u16,
    gv: u8,
    mv: u8,
    is: u8,
    it: u8,
    sp: u8,
    pwd: u8,
    pannings: [u8; 64],
    volumes: [u8; 64],
    orders: *mut [u8],
    instruments: *mut [ImpulseInstrument],
    samples: *mut [ImpulseSample],
    patterns: *mut [ImpulsePattern],
}

pub struct ImpulsePattern {
    rows: u16,
    data: *mut [u8],
}
#[repr(C)]
pub struct ImpulseSample {
    signatue: [u8; 4],
    filename: [u8; 12],
    _padding: u8,
    global_volume: u8,
    flags: u8,
    default_volume: u8,
    sample_name: [u8;26],
    cvt: u8,
    dfp: u8,
    length: u32,
    loop_begin: u32,
    loop_end: u32,
    mid_c_speed: u32,
    susloop_begin: u32,
    susloop_end: u32,
    sample_offset: *mut (),
    vibrato_speed: u8,
    vibrato_depth: u8,
    vibrato_rate: u8,
    vibrato_type: u8,
}
pub struct ImpulseInstrument {

}
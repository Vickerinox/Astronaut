use reboot_lib::sound::{timer_from_freq, RepeatMode, SoundControl, SoundFormat};

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
    reboot_lib::sound::timer_from_freq(4186 >> 7),
    reboot_lib::sound::timer_from_freq(4435 >> 7),
    reboot_lib::sound::timer_from_freq(4699 >> 7),
    reboot_lib::sound::timer_from_freq(4978 >> 7),
    reboot_lib::sound::timer_from_freq(5274 >> 7),
    reboot_lib::sound::timer_from_freq(5588 >> 7),
    reboot_lib::sound::timer_from_freq(5920 >> 7),
    reboot_lib::sound::timer_from_freq(6272 >> 7),
    reboot_lib::sound::timer_from_freq(6645 >> 7),
    reboot_lib::sound::timer_from_freq(7040 >> 7),
    reboot_lib::sound::timer_from_freq(7459 >> 7),
    reboot_lib::sound::timer_from_freq(7902 >> 7),
    //OCTAVE 1
    reboot_lib::sound::timer_from_freq(4186 >> 6),
    reboot_lib::sound::timer_from_freq(4435 >> 6),
    reboot_lib::sound::timer_from_freq(4699 >> 6),
    reboot_lib::sound::timer_from_freq(4978 >> 6),
    reboot_lib::sound::timer_from_freq(5274 >> 6),
    reboot_lib::sound::timer_from_freq(5588 >> 6),
    reboot_lib::sound::timer_from_freq(5920 >> 6),
    reboot_lib::sound::timer_from_freq(6272 >> 6),
    reboot_lib::sound::timer_from_freq(6645 >> 6),
    reboot_lib::sound::timer_from_freq(7040 >> 6),
    reboot_lib::sound::timer_from_freq(7459 >> 6),
    reboot_lib::sound::timer_from_freq(7902 >> 6),
    //OCTAVE 1
    reboot_lib::sound::timer_from_freq(4186 >> 5),
    reboot_lib::sound::timer_from_freq(4435 >> 5),
    reboot_lib::sound::timer_from_freq(4699 >> 5),
    reboot_lib::sound::timer_from_freq(4978 >> 5),
    reboot_lib::sound::timer_from_freq(5274 >> 5),
    reboot_lib::sound::timer_from_freq(5588 >> 5),
    reboot_lib::sound::timer_from_freq(5920 >> 5),
    reboot_lib::sound::timer_from_freq(6272 >> 5),
    reboot_lib::sound::timer_from_freq(6645 >> 5),
    reboot_lib::sound::timer_from_freq(7040 >> 5),
    reboot_lib::sound::timer_from_freq(7459 >> 5),
    reboot_lib::sound::timer_from_freq(7902 >> 5),
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


use common::music_mod::*;

pub struct MODPlayData {
    tick: u8,
    row: u8,
    frame: u8,
    current_song: *mut MODHeader,
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
                channel.timer.write(MUSIC_PITCHES[note as usize + 36]);

                let control = SoundControl::START
                    .with_repeat_mode(RepeatMode::Oneshot)
                    .with_sound_format(SoundFormat::PSG)
                    .with_panning(64)
                    .with_volume(volume >> 2)
                    | SoundControl::from_bits_retain(3 << 24);
                channel.control.write(control);

                if (MUSIC_COUNTER / 64) % 4 == 3 {
                    60
                } else {
                    72
                }
            } else {
                60
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

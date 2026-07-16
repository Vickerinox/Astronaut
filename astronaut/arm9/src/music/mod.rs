// SPDX-FileCopyrightText: 2026 Viktor Karlsson <viktor@koda.re>
// SPDX-License-Identifier: GPL-3.0-or-later

mod module;
mod wav;
pub use module::{send_mod_file, stop_mod_file};
use reboot_lib::music_modules::mods::MODAsyncLoader;
pub use wav::StreamingWav;
pub enum MusicPlaying {
    None,
    Mod(MODAsyncLoader),
    Wav(StreamingWav),
}

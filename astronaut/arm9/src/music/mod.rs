mod module;
mod wav;
use reboot_lib::music_modules::mods::MODAsyncLoader;
pub use wav::StreamingWav;
pub use module::{send_mod_file, stop_mod_file};
pub enum MusicPlaying {
    None,
    Mod(MODAsyncLoader),
    Wav(StreamingWav),
}

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

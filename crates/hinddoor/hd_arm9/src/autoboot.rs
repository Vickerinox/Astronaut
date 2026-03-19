use alloc::string::String;
use reboot_lib::Buttons;

pub enum Autoboot {
    NAND {
        category: u32,
        title_id: u32,
        game_version: u32,
        combo: Buttons,
    },
    SD {
        filepath: String,
        combo: Buttons,
    },
}

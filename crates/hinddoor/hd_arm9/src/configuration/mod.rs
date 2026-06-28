use alloc::string::{String, ToString};
use fatfs_embedded::fatfs::FileOptions;

pub struct Config {
    pub patch_flag: bool,
    pub autoboot: String,
    pub ini: String,
}
impl Config {
    pub const fn default() -> Self {
        Self { patch_flag: true, autoboot: String::new(), ini: String::new()}
    }
    pub fn load() -> Self {
        fatfs_embedded::open(
            &mut "sdmc:/_nds/vlaunch/settings.ini".to_string(),
            FileOptions::Read,
        ).ok().and_then(|mut file| {

            let size = fatfs_embedded::size(&mut file) as usize;
            let mut path_buf = alloc::vec![0u8; size];
            crate::boot::read_all(&mut path_buf, &mut file).ok()?;
            let str = String::from_utf8(path_buf).ok()?;
            let ini = ini::Ini::new(&str);

            let autoboot = ini.get("[boot]").and_then(|i| i.get("default")).unwrap_or("").to_string();
            let patch_flag = ini.get("[options]").and_then(|i| i.get("patching")).map(|i| i=="on").unwrap_or(true);
            Some(Config { patch_flag, autoboot, ini: str })
        }).unwrap_or(Config::default())
    }
}


pub mod ini;
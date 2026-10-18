// SPDX-FileCopyrightText: 2026 Viktor Karlsson <viktor@koda.re>
// SPDX-License-Identifier: GPL-3.0-or-later

use alloc::string::{String, ToString};

#[derive(Clone, Copy, PartialEq, PartialOrd)]
pub enum FileType {
    Dir,
    Rom,
    Mod,
    Wav,
    Bmp,
    Ini,
    Bin,
    None,
}

pub fn filetype(extension: &str) -> FileType {
    const ASSOCIATION_LIST: &[(&str, FileType)] = &[
        (".BIN", FileType::Bin),
        (".WAV", FileType::Wav),
        (".MOD", FileType::Mod),
        (".INI", FileType::Ini),
        (".BMP", FileType::Bmp),
        (".NDS", FileType::Rom),
        (".DSI", FileType::Rom),
        (".APP", FileType::Rom),
        (".SRL", FileType::Rom),
    ];
    ASSOCIATION_LIST
        .iter()
        .filter_map(|(t, i)| extension.ends_with(t).then_some(i))
        .next()
        .copied()
        .unwrap_or(FileType::None)
}
pub fn get_extension(str: &str) -> Option<&str> {
    let len = str.len().checked_sub(4)?;
    str.get(len..)
}

#[derive(Clone, PartialEq)]
pub struct FileEntry {
    pub file_name: String,
    pub display_name: String,
    pub kind: FileType,
}
impl PartialOrd for FileEntry {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        match self.kind.partial_cmp(&other.kind) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        self.display_name.partial_cmp(&other.display_name)
    }
}
pub fn truncate_name(string: &str, bound: usize) -> String {
    let mut string = string.to_string();
    if string.len() > bound + 3 {
        let mut boundary = bound;
        while !string.is_char_boundary(boundary) {
            boundary += 1;
        }
        string.truncate(boundary);
        string.push_str("...");
    }
    string
}

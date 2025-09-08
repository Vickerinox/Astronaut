pub struct MODHeader {
    pub song_name: [u8; 20],
    pub sample_info: [MODSampleData; 31],
    pub song_length: u8,
    _song_padding: u8,
    pub frames: [u8; 128],
    pub signature: [u8; 4],
    pub patterns: *mut [MODPattern],
    pub samples: [*mut [u8]; 31],
}
pub struct MODPattern {
    pub rows: [[[u8; 4]; 4]; 64],
}
pub struct MODSampleData {
    pub name: [u8; 22],
    pub length: u16,
    pub finetune: u8,
    pub volume: u8,
    pub repeat_point: u16,
    pub repeat_length: u16,
}
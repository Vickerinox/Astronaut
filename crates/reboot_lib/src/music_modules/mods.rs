use core::{mem, ops::Index, slice};

use alloc::{
    alloc::{alloc, dealloc, Layout},
    boxed::Box,
};

#[repr(C)]
#[derive(Debug)]
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

pub enum MODAsyncLoader {
    NotStarted(fatfs_embedded::fatfs::File),
    LoadingHeader {
        reader: fatfs_embedded::fatfs::File,
        progress: usize,
        wip_header: *mut [u8],
    },
    LoadingPatterns {
        reader: fatfs_embedded::fatfs::File,
        progress: usize,
        wip_patterns: *mut [u8],
        header: *mut MODHeader,
    },
    LoadingSamples {
        reader: fatfs_embedded::fatfs::File,
        progress: usize,
        sample: usize,
        header: *mut MODHeader,
    },
    Consumed,
}
impl MODAsyncLoader {
    pub fn new(reader: fatfs_embedded::fatfs::File) -> Self {
        Self::NotStarted(reader)
    }
    pub fn done(&self) -> bool {
        match self {
            Self::Consumed => true,
            _ => false,
        }
    }
    //perform one read, then return the result or ourselves needing to be re-processed.
    pub fn process(&mut self) -> Option<Box<MODHeader>> {
        let mut a = Self::Consumed;
        core::mem::swap(&mut a, self);
        *self = match a {
            MODAsyncLoader::NotStarted(mut reader) => {
                if fatfs_embedded::seek(&mut reader, 0) != Ok(()) {
                    *self = Self::Consumed;
                    return None;
                }
                let head_layout = unsafe {
                    Layout::from_size_align_unchecked(
                        mem::size_of::<MODHeader>(),
                        mem::align_of::<MODHeader>(),
                    )
                };
                let mut wip_header = unsafe {
                    slice::from_raw_parts_mut(alloc(head_layout), mem::size_of::<MODHeader>())
                };

                match fatfs_embedded::read(&mut reader, &mut wip_header[..1084]) {
                    Ok(progress) => {
                        Self::LoadingHeader {
                            reader: reader,
                            progress: progress as usize,
                            wip_header,
                        }
                    },
                    Err(err) => {
                        unsafe {
                            dealloc(wip_header as *mut [u8] as *mut u8, head_layout);
                        }
                        Self::Consumed
                    }
                }
            }
            MODAsyncLoader::LoadingHeader {
                mut reader,
                progress,
                mut wip_header,
            } => {
                let mut header = unsafe { &mut *wip_header };
                match fatfs_embedded::read(&mut reader, &mut header[progress..1084]) {
                    Ok(new_progress) => {
                        let progress = progress + new_progress as usize;
                        if progress == 1084 {
                            for remaining_value in &mut header[1084..] {
                                *remaining_value = 0;
                            }
                            let mut header =
                                unsafe { &mut *(wip_header as *mut u8 as *mut MODHeader) };
                            for sample_info in &mut header.sample_info {
                                sample_info.length = sample_info.length.to_be() << 1;
                                sample_info.repeat_length = sample_info.repeat_length.to_be();
                                sample_info.repeat_point = sample_info.repeat_point.to_be();
                            }
                            let wip_patterns = unsafe {
                                let max = header.frames.iter().max().copied().unwrap_or(0);
                                let len = max as usize + 1;
                                let allocation = alloc(Layout::from_size_align_unchecked(
                                    mem::size_of::<MODPattern>() * len,
                                    mem::align_of::<MODPattern>(),
                                ));
                                let mut slice = slice::from_raw_parts_mut(
                                    allocation,
                                    mem::size_of::<MODPattern>() * len,
                                );
                                slice
                            };
                            Self::LoadingPatterns {
                                reader,
                                progress: 0,
                                header,
                                wip_patterns,
                            }
                        } else {
                            Self::LoadingHeader {
                                reader,
                                progress,
                                wip_header,
                            }
                        }
                    }
                    Err(err) => {
                        unsafe {
                            let layout = Layout::from_size_align_unchecked(
                                mem::size_of::<MODHeader>(),
                                mem::align_of::<MODHeader>(),
                            );
                            dealloc(wip_header as *mut u8, layout);
                        }
                        Self::Consumed
                    }
                }
            }
            MODAsyncLoader::LoadingPatterns {
                mut reader,
                progress,
                mut header,
                wip_patterns,
            } => {
                let mut patterns = unsafe {&mut *wip_patterns };
                match fatfs_embedded::read(&mut reader, &mut patterns[progress..]) {
                    Ok(new_progress) => {
                        let progress = progress + new_progress as usize;
                        if progress == patterns.len() {
                            let (ptr, len) = wip_patterns.to_raw_parts();
                            unsafe {
                                let patterns = slice::from_raw_parts_mut(
                                    ptr as *mut MODPattern,
                                    len / mem::size_of::<MODPattern>(),
                                );
                                (*header).patterns = patterns;
                            }
                            Self::LoadingSamples {
                                reader,
                                progress: 0,
                                sample: 0,
                                header,
                            }
                        } else {
                            Self::LoadingPatterns {
                                reader,
                                progress,
                                wip_patterns,
                                header,
                            }
                        }
                    }
                    Err(err) => {
                        unsafe {
                            let layout = Layout::from_size_align_unchecked(
                                wip_patterns.len(),
                                mem::align_of::<MODPattern>(),
                            );
                            dealloc(wip_patterns as *mut u8, layout);
                        }
                        Self::Consumed
                    }
                }
            }
            MODAsyncLoader::LoadingSamples {
                mut reader,
                mut progress,
                mut header,
                mut sample,
            } => {
                let header = unsafe { &mut *header };
                loop {
                    let Some(info) = header.sample_info.get(sample) else {
                        unsafe {
                            crate::flush_mmc();
                            *self = Self::Consumed;
                            return Some(alloc::boxed::Box::from_raw(header))
                        }
                    };
                    if info.length > 0 {
                        let mut buffer = match unsafe { header.samples[sample].as_mut() } {
                            None => {
                                let new_buffer = unsafe {
                                    let sample_len = info.length as usize;
                                    //needs align 4 due to hardware limitations
                                    let sample_buffer =
                                        alloc(Layout::from_size_align_unchecked(sample_len, 4));
                                    slice::from_raw_parts_mut(sample_buffer, sample_len)
                                };
                                header.samples[sample] = new_buffer;
                                new_buffer
                            }
                            Some(valid) => valid,
                        };
                        match fatfs_embedded::read(&mut reader, &mut buffer[progress..]) {
                            Ok(new_progress) => {
                                progress += new_progress as usize;
                                if progress == buffer.len() {
                                    sample += 1;
                                    progress = 0;
                                }
                                break Self::LoadingSamples {
                                    reader,
                                    progress,
                                    sample,
                                    header,
                                }        
                            }
                            Err(err) => {
                                break Self::Consumed
                            },
                        };
                    } else {
                        progress = 0;
                        sample += 1;
                    }
                }
                //we've gone through all samples!
                
            }
            other => other,
        };
        None
    }
}
impl MODAsyncLoader {
    pub fn progress(&self) -> (usize, usize) {
        match self {
            MODAsyncLoader::NotStarted(_) => (0, usize::MAX),
            MODAsyncLoader::LoadingHeader { progress, .. } => (*progress, usize::MAX),
            MODAsyncLoader::LoadingPatterns {
                progress,
                wip_patterns,
                header,
                ..
            } => {
                let header_len = 1084;
                let header = unsafe { &mut **header };
                let sample_len: usize = header.sample_info.iter().map(|x| x.length as usize).sum();
                let pattern_len = wip_patterns.len();
                (
                    header_len + *progress,
                    header_len + sample_len + pattern_len,
                )
            }
            MODAsyncLoader::LoadingSamples {
                progress,
                sample,
                header,
                ..
            } => {
                let header = unsafe { &mut **header };
                let header_len = 1084;
                let sample_len: usize = header.sample_info.iter().map(|x| x.length as usize).sum();
                let pattern_len = header.patterns.len() * mem::size_of::<MODPattern>();
                let found_samples_len: usize =
                    header.samples[..*sample].iter().map(|i| i.len()).sum();
                (
                    header_len + pattern_len + found_samples_len + *progress,
                    header_len + sample_len + pattern_len,
                )
            }
            _ => (0, usize::MAX),
        }
    }
}
impl core::default::Default for MODHeader {
    fn default() -> Self {
        #[allow(invalid_null_arguments)]
        Self {
            song_name: Default::default(),
            sample_info: Default::default(),
            song_length: Default::default(),
            _song_padding: Default::default(),
            frames: [0u8; 128],
            signature: Default::default(),
            patterns: unsafe { slice::from_raw_parts_mut(core::ptr::null_mut(), 0) },
            samples: [unsafe { slice::from_raw_parts_mut(core::ptr::null_mut(), 0) }; 31],
        }
    }
}
impl Drop for MODHeader {
    fn drop(&mut self) {
        let Self {
            song_name,
            sample_info,
            song_length,
            _song_padding,
            frames,
            signature,
            patterns,
            samples,
        } = self;
        for sample in samples {
            if !sample.is_null() {
                unsafe {
                    dealloc(
                        *sample as *mut u8,
                        Layout::from_size_align_unchecked(sample.len(), 4),
                    );
                }
            }
        }
        unsafe {
            if !patterns.is_null() {
                let pattern_layout = Layout::from_size_align_unchecked(
                    mem::size_of::<MODPattern>() * patterns.len(),
                    mem::align_of::<MODPattern>(),
                );
                dealloc(*patterns as *mut MODPattern as *mut u8, pattern_layout);
            }
        }
    }
}
#[repr(C)]
pub struct MODPattern([[MODROW; 4]; 64]);
impl Index<usize> for MODPattern {
    type Output = [MODROW; 4];

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}
#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct MODROW([u8; 4]);
impl MODROW {
    pub fn period(&self) -> u16 {
        self.0[1] as u16 | ((self.0[0] as u16 & 0xf) << 8)
    }
    pub fn sample_index(&self) -> u8 {
        (self.0[0] & 0xF0) | ((self.0[2] & 0xF0) >> 4)
    }
    pub fn command(&self) -> Command {
        Command {
            kind: self.0[2] & 0xF,
            value: self.0[3],
        }
    }
}
pub struct Command {
    pub kind: u8,
    pub value: u8,
}

#[repr(C)]
#[derive(Debug, Default)]
pub struct MODSampleData {
    pub name: [u8; 22],
    pub length: u16,
    pub finetune: u8,
    pub volume: u8,
    pub repeat_point: u16,
    pub repeat_length: u16,
}

impl MODPattern {}
/*
impl MODSampleData {
    pub fn read_from<R: fatfs::Read>(reader: &mut R) -> Result<Self, R::Error> {
        let mut name = [0u8; 22];
        reader.read_exact(&mut name)?;

        let mut length = [0u8; 2];
        reader.read_exact(&mut length)?;

        let mut finetune = [0u8; 1];
        reader.read_exact(&mut finetune)?;

        let mut volume = [0u8; 1];
        reader.read_exact(&mut volume)?;

        let mut repeat_point = [0u8; 2];
        reader.read_exact(&mut repeat_point)?;

        let mut repeat_length = [0u8; 2];
        reader.read_exact(&mut repeat_length)?;

        let mut length = u16::from_be_bytes(length) << 1;
        let mut finetune = finetune[0];
        let mut volume = volume[0];
        let mut repeat_point = u16::from_be_bytes(repeat_point);
        let mut repeat_length = u16::from_be_bytes(repeat_length);

        Ok(Self {
            name,
            length,
            finetune,
            volume,
            repeat_point,
            repeat_length,
        })
    }
}

impl MODHeader {
    pub fn read_from<R: fatfs::Read + fatfs::Seek>(mut reader: R) -> Result<MODHeader, R::Error> {
        assert!(reader.seek(fatfs::SeekFrom::Current(0)).ok() == Some(0));
        let mut ourself = Self::default();
        reader.read_exact(&mut ourself.song_name)?;
        for sample in ourself.sample_info.iter_mut() {
            *sample = MODSampleData::read_from(&mut reader)?;
        }

        let mut song_length = [0u8; 1];
        let mut _pad = [0u8; 1];
        reader.read_exact(&mut song_length)?;
        ourself.song_length = song_length[0];
        reader.read_exact(&mut _pad)?;

        reader.read_exact(&mut ourself.frames)?;
        reader.read_exact(&mut ourself.signature)?;
        let max = ourself.frames.iter().max().copied().unwrap_or(0);
        let patterns = unsafe {
            let len = max as usize + 1;
            let allocation = alloc(Layout::from_size_align_unchecked(
                mem::size_of::<MODPattern>() * len,
                mem::align_of::<MODPattern>(),
            ));
            let mut slice =
                slice::from_raw_parts_mut(allocation, mem::size_of::<MODPattern>() * len);
            reader.read_exact(slice);
            slice::from_raw_parts_mut(allocation as *mut MODPattern, len)
        };
        ourself.patterns = patterns;

        for (i, sample) in ourself.sample_info.iter().enumerate() {
            let sample_len = sample.length as usize;
            let slice = unsafe {
                //needs align 4 due to hardware limitations
                let sample_buffer = alloc(Layout::from_size_align_unchecked(sample_len, 4));
                slice::from_raw_parts_mut(sample_buffer, sample_len)
            };
            reader.read_exact(slice)?;
            ourself.samples[i] = slice;
        }
        Ok(ourself)
    }
}
*/

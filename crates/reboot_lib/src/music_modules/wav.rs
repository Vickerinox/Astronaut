pub struct WavHeader {
    channels: u16,
    sample_frequency: u32,
    sample_type: SampleType,
}
pub enum SampleType {
    Unsigned8Bit,
    Unsigned16Bit,
}

pub struct WavPlay {
    header: WavHeader,
    scratch_buffer: *mut (),
    len: usize,
}

//DKA bootstub struct
const _BOOTSTUB_MAGIC: u64 = 0x62757473746F6F62; // "bootstub"
const _BOOTSTUB_LOCATION: *mut BootStub = 0x2FF4000 as *mut BootStub;
#[repr(C)]
pub struct BootStub {
    pub magic: u64,
    pub arm9_entry: *const (),
    pub arm7_entry: *const (),
    pub loader_size: u32,
}
pub fn install() {}

pub struct File {
    path: alloc::string::String,
    contents: Option<alloc::vec::Vec<u8>>,
    metadata: Option<alloc::vec::Vec<u8>>,
}

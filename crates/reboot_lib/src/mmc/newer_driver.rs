pub trait SDMMCCommand {
    type Response: SDMMCResponse;
    fn cmd_nr(&self) -> u32;
}
pub trait SDMMCResponse: Default {
    fn gather_response(&mut self, full_response: &[u32]);
}

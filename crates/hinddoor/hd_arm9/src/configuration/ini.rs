use alloc::vec::Vec;

pub struct Ini<'a> {
    segments: Vec<(&'a str, Segment<'a>)>,
}
#[derive(Default)]
pub struct Segment<'a>(Vec<Entry<'a>>);
pub enum Entry<'a> {
    Comment(&'a str),
    Value(&'a str, &'a str, &'a str),
}

impl<'a> Ini<'a> {
    pub fn new(from: &'a str, mut key_fn: Option<&mut dyn FnMut(&'a str, &'a str, &'a str)>) -> Self {
        let mut segment_name = "";
        let mut segment = Segment(Vec::new());
        let mut segments = Vec::new();
        for line in from.split(['\n'].as_slice()) {
            let mut parts = line.split([';', '#'].as_slice());
            let Some(values) = parts.next() else { continue };
            let values = values.trim();
            let comment = parts.remainder().unwrap_or("");
            let mut chars = values.chars();
            match chars.next() {
                Some('[') => {
                    if chars.last() == Some(']') {
                        segments.push((segment_name, core::mem::take(&mut segment)));
                        segment_name = values;
                    }
                }
                None => {
                    segment.0.push(Entry::Comment(comment));
                }
                _ => {
                    let mut split = values.split(['='].as_slice());
                    let Some(key) = split.next() else { continue };
                    let Some(value) = split.next() else { continue };
                    let key = key.trim();
                    let value = value.trim();
                    if split.next().is_some() {
                        continue;
                    };
                    if let Some(func) = &mut key_fn {
                        func(&segment_name, key, value)
                    }
                    segment
                        .0
                        .push(Entry::Value(key, value, comment.trim()));
                }
            }
        }
        segments.push((segment_name, core::mem::take(&mut segment)));
        Self { segments }
    }
    pub fn get(&self, key: &str) -> Option<&Segment<'_>> {
        self.segments.iter().find(|i| i.0 == key).map(|i| &i.1)
    }
}
impl<'a> Segment<'a> {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.0
            .iter()
            .filter_map(|i| match i {
                Entry::Comment(_) => None,
                Entry::Value(key2, value, _comment) => (*key2 == key).then_some(*value),
            })
            .next()
    }
}

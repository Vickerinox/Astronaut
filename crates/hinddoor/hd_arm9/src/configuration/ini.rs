use alloc::{string::String, vec::Vec};

pub struct Ini<'a> {
    segments: Vec<(&'a str, Segment<'a>)>,
}
#[derive(Default)]
pub struct Segment<'a>(Vec<Entry<'a>>);
pub enum Entry<'a> {
    Comment(&'a str),
    Value(&'a str, &'a str),
}

impl<'a> Ini<'a> {
    pub fn new(from: &'a str) -> Self {
        let mut segment_name = "";
        let mut segment = Segment(Vec::new());
        let mut segments = Vec::new();
        for line in from.split(['\n', '\r']) {
            let trimmer = line.trim();
            let mut a = trimmer.chars();
            match a.next() {
                Some(';') | Some('#') => {
                    segment.0.push(Entry::Comment(trimmer));
                }
                Some('[') => {
                    if a.last() == Some(']') {
                        segments.push((segment_name, core::mem::take(&mut segment)));
                        segment_name = trimmer;
                    }
                }
                None => { continue },
                _ => {
                    let mut split = trimmer.split('=');
                    let Some(key) = split.next() else { continue };
                    let Some(value) = split.next() else {continue};
                    if split.next().is_some() { continue};
                    segment.0.push(Entry::Value(key.trim(), value.trim()));
                }
            }
        }
        segments.push((segment_name, core::mem::take(&mut segment)));
        Self {segments }
    }
    pub fn get(&self, key: &str) -> Option<&Segment> {
        self.segments.iter().find(|i| i.0 == key).map(|i| &i.1)
    }
}
impl<'a> Segment<'a> {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.0.iter().filter_map(|i| match i {
            Entry::Comment(_) => None,
            Entry::Value(key2, value) => (*key2==key).then_some(*value),
        }).next()
    }
}

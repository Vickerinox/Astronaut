pub struct Ini {}

impl Ini {
    pub fn new<'a>(from: &'a str, mut key_fn: Option<&mut dyn FnMut(&'a str, &'a str, &'a str)>) {
        let mut segment_name = "";
        for line in from.split(['\n'].as_slice()) {
            let mut parts = line.split([';', '#'].as_slice());
            let Some(values) = parts.next() else { continue };
            let values = values.trim();
            let mut chars = values.chars();
            match chars.next() {
                Some('[') => {
                    if chars.last() == Some(']') {
                        segment_name = values;
                    }
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
                }
            }
        }
    }
}

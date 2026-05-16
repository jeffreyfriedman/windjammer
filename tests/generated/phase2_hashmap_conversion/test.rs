use std::collections::HashMap;
#[derive(Debug, Clone, PartialEq)]
#[repr(C)]
pub struct Cache {
    pub data: HashMap<String, i32>,
}

impl Cache {
#[inline]
pub fn store(&mut self, key: &str, value: i32) {
        self.data.insert(key.to_string(), value);
}
}


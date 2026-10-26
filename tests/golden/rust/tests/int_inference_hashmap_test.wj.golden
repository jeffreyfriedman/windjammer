use windjammer_runtime::test::*;

use std::collections::HashMap;
#[derive(Debug, Clone, PartialEq)]
#[repr(C)]
pub struct TestStruct {
    pub data: HashMap<u32, String>,
}

impl TestStruct {
#[inline]
pub fn new() -> TestStruct {
        TestStruct { data: HashMap::new() }
}
#[inline]
pub fn insert_test(&mut self, key: u32, value: String) {
        self.data.insert(key, value);
}
#[inline]
pub fn get_test(&self, key: u32) -> Option<String> {
        self.data.get(&key).cloned()
}
}

#[test]

pub fn test_with_local() {
    let mut map: HashMap<u32, String> = HashMap::new();
    let key: u32 = 42_u32;
    map.insert(key, String::from("test"));
    let _result = map.get(&key);
}


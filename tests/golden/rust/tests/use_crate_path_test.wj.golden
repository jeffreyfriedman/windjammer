use crate::ffi;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(C)]
pub struct TestStruct {
    pub value: i64,
}
impl TestStruct {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut __bytes = Vec::with_capacity(8);
        __bytes.extend_from_slice(&self.value.to_ne_bytes());
        __bytes
    }
}


impl TestStruct {
#[inline]
pub fn new() -> TestStruct {
        TestStruct { value: 42_i64 }
}
}


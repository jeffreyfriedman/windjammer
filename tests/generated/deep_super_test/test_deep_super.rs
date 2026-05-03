use super::super::super::core::types::MyType;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(C)]
pub struct MyType {
    pub value: i32,
}
impl MyType {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut __bytes = Vec::with_capacity(4);
        __bytes.extend_from_slice(&self.value.to_ne_bytes());
        __bytes
    }
}


#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Foo {
    pub value: MyType,
}


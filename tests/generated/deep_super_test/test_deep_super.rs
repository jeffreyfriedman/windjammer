use super::MyType;

#[derive(Copy, Clone, Debug)]
pub struct MyType {
    pub value: i32,
}

#[derive(Debug, Clone, Copy)]
pub struct Foo {
    pub value: MyType,
}


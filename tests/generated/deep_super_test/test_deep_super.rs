use super::MyType;

#[derive(Copy, Clone, Debug)]
pub struct MyType {
    pub value: i32,
}

#[derive(Debug, Clone)]
pub struct Foo {
    pub value: MyType,
}


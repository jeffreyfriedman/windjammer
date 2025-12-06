use super::types::MyType;


#[derive(Copy, Clone, Debug)]
pub struct MyType {
    pub value: i32,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Foo {
    pub value: MyType,
}


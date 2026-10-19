pub trait Calculator {
    fn add(&self, a: f32, b: f32) -> f32 {
        a + b
    }
    fn multiply(&self, x: i64, y: i64) -> i64 {
        x * y
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[repr(C)]
pub struct SimpleCalc {
    pub result: f32,
}
impl SimpleCalc {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut __bytes = Vec::with_capacity(4);
        __bytes.extend_from_slice(&self.result.to_ne_bytes());
        __bytes
    }
}


impl Calculator for SimpleCalc {
#[inline]
fn add(&self, a: f32, b: f32) -> f32 {
        a + b
}
#[inline]
fn multiply(&self, x: i64, y: i64) -> i64 {
        x * y
}
}


pub trait Named {
    fn label(&self) -> String;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]

struct Circle {
    r: i64,
}
impl Circle {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut __bytes = Vec::with_capacity(8);
        __bytes.extend_from_slice(&self.r.to_ne_bytes());
        __bytes
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]

struct Square {
    s: i64,
}
impl Square {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut __bytes = Vec::with_capacity(8);
        __bytes.extend_from_slice(&self.s.to_ne_bytes());
        __bytes
    }
}


impl Named for Circle {
#[inline]
fn label(&self) -> String {
        format!("Circle r={}", self.r)
}
}

impl Named for Square {
#[inline]
fn label(&self) -> String {
        format!("Square s={}", self.s)
}
}

#[inline]
fn print_label<T: Named>(x: &T) {
    println!("{}", x.label());
}

fn main() {
    print_label(&Circle { r: 2_i64 });
    print_label(&Square { s: 3_i64 });
}


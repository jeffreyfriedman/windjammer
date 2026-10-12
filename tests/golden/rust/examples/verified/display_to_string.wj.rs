#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]

struct Point {
    x: i64,
    y: i64,
}










impl Point {
#[inline]
fn to_string(&self) -> String {
        format!("({}, {})", self.x, self.y)
}
}

impl std::fmt::Display for Point {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&Point::to_string(self))
    }
}

fn main() {
    let p = Point { x: 1_i64, y: 2_i64 };
    println!("{}", p);
    let s = format!("at {}!", p);
    println!("{}", s);
    let q = Point { x: 3_i64, y: 4_i64 };
    println!("[p={}]", q);
}


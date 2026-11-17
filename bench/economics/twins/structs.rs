// Rust twin of bench/interp/structs.wj: methods on a small Copy struct.
#[derive(Clone, Copy)]
struct V2 {
    x: i64,
    y: i64,
}

impl V2 {
    fn add(self, o: V2) -> V2 {
        V2 { x: self.x + o.x, y: self.y + o.y }
    }
    fn dot(self, o: V2) -> i64 {
        self.x * o.x + self.y * o.y
    }
}

fn main() {
    let mut acc = V2 { x: 0, y: 0 };
    let mut s = 0;
    for i in 0..300_000 {
        let v = V2 { x: i % 13, y: i % 7 };
        acc = acc.add(v);
        s += acc.dot(v) % 1000;
    }
    println!("{} {} {}", acc.x, acc.y, s);
}

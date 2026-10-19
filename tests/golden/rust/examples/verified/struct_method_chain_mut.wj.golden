#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]

struct Counter {
    n: i64,
    step: i64,
}










impl Counter {
#[inline]
fn tick(&mut self) {
        self.n += self.step;
}
#[inline]
fn value(&self) -> i64 {
        self.n
}
}

#[inline]
fn run(c: &mut Counter) -> i64 {
    c.tick();
    c.tick();
    c.value()
}

fn main() {
    let mut a = Counter { n: 1_i64, step: 1_i64 };
    a.tick();
    a.tick();
    println!("{}", a.value());
    let mut b = Counter { n: 0_i64, step: 15_i64 };
    println!("{}", run(&mut b));
}


#[derive(Clone, Debug)]
enum Slot<T> {
    Full(T),
    Empty,
}

#[inline]
fn show(s: Slot<i64>) -> String {
    match s {
        Slot::Full(v) => format!("Full({})", v),
        Slot::Empty => String::from("Empty"),
    }
}

#[inline]
fn or_zero(s: Slot<i64>) -> i64 {
    match s {
        Slot::Full(v) => v,
        Slot::Empty => 0_i64,
    }
}

fn main() {
    println!("{}", show(Slot::Full(7)));
    println!("{}", show(Slot::Empty));
    println!("{}", or_zero(Slot::Full(7)));
}


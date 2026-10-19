#[inline]
fn largest<T: PartialOrd>(items: Vec<T>) -> T {
    let mut best = items[0].clone();
    for i in items {
        if i > best {
            best = i;
        }
    }
    best
}

#[inline]
fn reversed<T>(items: Vec<T>) -> Vec<T> {
    let mut out = vec![];
    for i in items.into_iter().rev() {
        out.push(i);
    }
    out
}

fn main() {
    println!("{}", largest([3, 9, 4]));
    println!("{}", largest(["a", "z", "m"]));
    println!("{:?}", reversed([1, 2, 3]));
}


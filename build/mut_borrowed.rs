#[inline]
fn increment(x: &mut i64) {
    x += 1;
}

fn main() {
    let mut counter = 0;
    increment(&mut counter);
    println!("Counter: {}", counter)
}


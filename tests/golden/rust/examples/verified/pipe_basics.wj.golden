#[inline]
fn double(x: i64) -> i64 {
    x * 2_i64
}

#[inline]
fn add(x: i64, y: i64) -> i64 {
    x + y
}

#[inline]
fn shout(s: &str) -> String {
    s.to_uppercase() + &String::from("!")
}

fn main() {
    let a = double(add(3_i64, 5_i64));
    println!("{}", a);
    let b = add(double(4_i64), 10_i64);
    println!("{}", b);
    println!("{}", shout(&greet("ada")));
}

#[inline]
fn greet(name: &str) -> String {
    format!("hello {}", name)
}


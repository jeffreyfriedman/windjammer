#[inline]
fn gcd(a: i64, b: i64) -> i64 {
    if b == 0_i64 {
        a
    } else {
        gcd(b, a % b)
    }
}

#[inline]
fn lcm(a: i64, b: i64) -> i64 {
    a / gcd(a, b) * b
}

fn main() {
    println!("gcd(48, 18) = {}", gcd(48_i64, 18_i64));
    println!("gcd(17, 5) = {}", gcd(17_i64, 5_i64));
    println!("lcm(48, 18) = {}", lcm(48_i64, 18_i64));
}


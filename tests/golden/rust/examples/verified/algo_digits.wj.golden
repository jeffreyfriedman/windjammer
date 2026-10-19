#[inline]
fn digit_sum(n: i64) -> i64 {
    let mut x = n;
    let mut s = 0_i64;
    while x > 0_i64 {
        s += x % 10_i64;
        x /= 10;
    }
    s
}

fn to_binary(n: i64) -> String {
    if n == 0_i64 {
        return String::from("0");
    }
    let mut x = n;
    let mut digits: Vec<String> = vec![];
    while x > 0_i64 {
        let _ = digits.push((x % 2_i64).to_string());
        x /= 2;
    }
    digits.reverse();
    digits.join("")
}

#[inline]
fn from_binary(s: &str) -> i64 {
    let mut n = 0_i64;
    for c in s.chars() {
        n = n * 2_i64 + c.to_digit(2).unwrap_or(0) as i64;
    }
    n
}

fn main() {
    println!("{}", digit_sum(98765_i64));
    println!("{}", from_binary("1011"));
    println!("{}", to_binary(37_i64));
}


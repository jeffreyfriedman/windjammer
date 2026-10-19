#[inline]
fn shift(s: &str, k: i64) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c >= 'a' && c <= 'z' {
            let base = 'a' as i64;
            let moved = ((c as i64 - base + k) % 26_i64 + 26_i64) % 26_i64 + base;
            out.push(moved as u8 as char)
        } else {
            out.push(c)
        }
    }
    out
}

fn main() {
    let enc = shift("hello, world", 3_i64);
    println!("{}", enc);
    println!("{}", shift(&enc, -3_i64));
}


#[inline]
fn build(words: Vec<String>) -> String {
    let mut out = String::new();
    for w in words {
        out = format!("{}{}{}", out, w, ",");
    }
    out
}

fn main() {
    let mut words: Vec<String> = Vec::new();
    for i in 0_i32..40000_i32 {
        words.push(i.to_string());
    }
    let s = build(words);
    let mut h: u64 = 1469598103934665603_u64;
    for b in s.bytes() {
        h = (h ^ (b as u64)).wrapping_mul(1099511628211_u64);
    }
    println!("{} {}", s.len(), h);
}


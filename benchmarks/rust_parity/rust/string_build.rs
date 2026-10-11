fn build(words: &[String]) -> String {
    let mut out = String::with_capacity(words.iter().map(|w| w.len() + 1).sum());
    for w in words {
        out.push_str(w);
        out.push(',');
    }
    out
}

fn main() {
    let words: Vec<String> = (0..40000).map(|i: i32| i.to_string()).collect();
    let s = build(&words);
    let mut h: u64 = 1469598103934665603;
    for b in s.bytes() {
        h = (h ^ b as u64).wrapping_mul(1099511628211);
    }
    println!("{} {}", s.len(), h);
}

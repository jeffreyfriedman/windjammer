use std::collections::HashMap;
use std::io::Read;

fn main() {
    let mut buf = Vec::new();
    std::io::stdin().read_to_end(&mut buf).unwrap();
    let nl = buf.iter().position(|&b| b == b'\n').unwrap_or(buf.len());
    let k: usize = String::from_utf8_lossy(&buf[..nl]).trim().parse().unwrap();
    let rest = if nl < buf.len() { &buf[nl + 1..] } else { &buf[..0] };
    let mut counts: HashMap<Vec<u8>, u64> = HashMap::new();
    for w in rest.split(|b| !b.is_ascii_alphanumeric()).filter(|w| !w.is_empty()) {
        *counts.entry(w.to_ascii_lowercase()).or_insert(0) += 1;
    }
    let mut items: Vec<(Vec<u8>, u64)> = counts.into_iter().collect();
    items.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    for (w, c) in items.into_iter().take(k) {
        println!("{} {}", String::from_utf8_lossy(&w), c);
    }
}

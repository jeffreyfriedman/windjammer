use std::io::Read;

fn analyze(line: &str) -> (usize, String, usize) {
    let c: Vec<char> = line.chars().collect();
    let n = c.len();
    let mut best = (0usize, 0usize);
    let mut count = 0;
    // expand around every center
    for center in 0..(2 * n).saturating_sub(1) {
        let mut l = (center / 2) as isize;
        let mut r = (center / 2 + center % 2) as isize;
        while l >= 0 && (r as usize) < n && c[l as usize] == c[r as usize] {
            count += 1;
            let len = (r - l + 1) as usize;
            let start = l as usize;
            if len > best.0 || (len == best.0 && start < best.1) {
                best = (len, start);
            }
            l -= 1;
            r += 1;
        }
    }
    (best.0, c[best.1..best.1 + best.0].iter().collect(), count)
}

fn main() {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    for line in s.lines() {
        let (len, sub, count) = analyze(line.trim_end_matches('\r'));
        if len == 0 {
            println!("0 - 0");
        } else {
            println!("{} {} {}", len, sub, count);
        }
    }
}

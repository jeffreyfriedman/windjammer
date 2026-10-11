use std::io::Read;

fn counts(data: &[u8]) -> (u64, u64, u64) {
    let lines = data.iter().filter(|&&b| b == b'\n').count() as u64;
    let mut words = 0;
    let mut inword = false;
    for &b in data {
        let ws = matches!(b, b' ' | b'\t' | b'\n' | b'\r');
        if !ws && !inword {
            words += 1;
        }
        inword = !ws;
    }
    (lines, words, data.len() as u64)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        let mut d = Vec::new();
        std::io::stdin().read_to_end(&mut d).unwrap();
        let (l, w, b) = counts(&d);
        println!("{} {} {}", l, w, b);
        return;
    }
    let mut tot = (0, 0, 0);
    let mut failed = false;
    for a in &args {
        match std::fs::read(a) {
            Ok(d) => {
                let (l, w, b) = counts(&d);
                println!("{} {} {} {}", l, w, b, a);
                tot = (tot.0 + l, tot.1 + w, tot.2 + b);
            }
            Err(_) => {
                println!("wc: {}: No such file", a);
                failed = true;
            }
        }
    }
    if args.len() > 1 {
        println!("{} {} {} total", tot.0, tot.1, tot.2);
    }
    std::process::exit(if failed { 1 } else { 0 });
}

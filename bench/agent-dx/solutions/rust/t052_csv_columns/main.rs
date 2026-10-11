fn valid(v: &str) -> Option<u128> {
    if v.is_empty() || v.len() > 30 || !v.bytes().all(|b| b.is_ascii_digit()) {
        None
    } else {
        v.parse().ok()
    }
}

fn die(msg: String, code: i32) -> ! {
    println!("{}", msg);
    std::process::exit(code);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 3 {
        die("usage: csv FILE COLUMN OP".into(), 2);
    }
    let (file, col, op) = (&args[0], &args[1], args[2].as_str());
    let text = match std::fs::read_to_string(file) {
        Ok(t) => t,
        Err(_) => die(format!("error: cannot read {}", file), 1),
    };
    let mut lines = text.lines().enumerate();
    let header: Vec<&str> = lines.next().map(|(_, h)| h.split(',').collect()).unwrap_or_default();
    let Some(idx) = header.iter().position(|h| h == col) else {
        die(format!("error: no such column {}", col), 2)
    };
    if !["sum", "min", "max", "count", "mean"].contains(&op) {
        die(format!("error: unknown op {}", op), 2);
    }
    let rows: Vec<(usize, &str)> = lines
        .filter(|(_, l)| !l.is_empty())
        .map(|(i, l)| (i + 1, l.split(',').nth(idx).unwrap_or("")))
        .collect();
    if op == "count" {
        println!("{}", rows.len());
        return;
    }
    let mut nums: Vec<u128> = Vec::new();
    for (n, v) in &rows {
        match valid(v) {
            Some(x) => nums.push(x),
            None => die(format!("error: bad value {} on line {}", v, n), 3),
        }
    }
    if nums.is_empty() {
        println!("{}", if op == "sum" { "0" } else { "n/a" });
        return;
    }
    let sum: u128 = nums.iter().sum();
    let r = match op {
        "sum" => sum,
        "min" => *nums.iter().min().unwrap(),
        "max" => *nums.iter().max().unwrap(),
        _ => sum / nums.len() as u128,
    };
    println!("{}", r);
}

struct Opts {
    name: String,
    count: u64,
    verbosity: u32,
    dry_run: bool,
    positional: Vec<String>,
}

fn parse_count(v: &str) -> Result<u64, String> {
    if !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit()) {
        let t = v.trim_start_matches('0');
        if t.is_empty() {
            return Ok(0);
        }
        if let Ok(n) = t.parse() {
            return Ok(n);
        }
    }
    Err(format!("error: invalid count {}", v))
}

fn parse(args: &[String]) -> Result<Opts, String> {
    let mut o = Opts { name: "anonymous".into(), count: 1, verbosity: 0, dry_run: false, positional: vec![] };
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        i += 1;
        if a == "--" {
            o.positional.extend(args[i..].iter().cloned());
            break;
        } else if let Some(v) = a.strip_prefix("--name=") {
            o.name = v.to_string();
        } else if let Some(v) = a.strip_prefix("--count=") {
            o.count = parse_count(v)?;
        } else if a == "--name" || a == "--count" {
            if i >= args.len() {
                return Err(format!("error: missing value for {}", a));
            }
            let v = args[i].as_str();
            i += 1;
            if a == "--name" {
                o.name = v.to_string();
            } else {
                o.count = parse_count(v)?;
            }
        } else if a == "--dry-run" {
            o.dry_run = true;
        } else if a.len() >= 2 && a.starts_with('-') && a[1..].bytes().all(|b| b == b'v') {
            o.verbosity += (a.len() - 1) as u32;
        } else if a.starts_with('-') && a != "-" {
            return Err(format!("error: unknown option {}", a));
        } else {
            o.positional.push(a.to_string());
        }
    }
    Ok(o)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match parse(&args) {
        Ok(o) => {
            println!("name: {}", o.name);
            println!("count: {}", o.count);
            println!("verbosity: {}", o.verbosity);
            println!("dry-run: {}", o.dry_run);
            println!("positional: {}", o.positional.join(" "));
        }
        Err(e) => {
            println!("{}", e);
            std::process::exit(2);
        }
    }
}

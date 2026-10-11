fn main() {
    let (mut ci, mut cn, mut cc, mut cv) = (false, false, false, false);
    let mut rest: Vec<String> = Vec::new();
    for a in std::env::args().skip(1) {
        match a.as_str() {
            "-i" => ci = true,
            "-n" => cn = true,
            "-c" => cc = true,
            "-v" => cv = true,
            _ => rest.push(a),
        }
    }
    if rest.is_empty() {
        std::process::exit(2);
    }
    let pat = rest.remove(0);
    let needle = if ci { pat.to_ascii_lowercase() } else { pat };
    let multi = rest.len() > 1;
    let mut total = 0u64;
    let mut missing = false;
    for f in &rest {
        let data = match std::fs::read(f) {
            Ok(d) => String::from_utf8_lossy(&d).into_owned(),
            Err(_) => {
                println!("grep: {}: No such file", f);
                missing = true;
                continue;
            }
        };
        let mut count = 0u64;
        for (i, line) in data.lines().enumerate() {
            let hay = if ci { line.to_ascii_lowercase() } else { line.to_string() };
            if hay.contains(&needle) != cv {
                count += 1;
                if !cc {
                    let mut out = String::new();
                    if multi {
                        out += &format!("{}:", f);
                    }
                    if cn {
                        out += &format!("{}:", i + 1);
                    }
                    println!("{}{}", out, line);
                }
            }
        }
        total += count;
        if cc {
            if multi {
                println!("{}:{}", f, count);
            } else {
                println!("{}", count);
            }
        }
    }
    let code = if missing { 2 } else if total > 0 { 0 } else { 1 };
    std::process::exit(code);
}

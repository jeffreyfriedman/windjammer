use std::io::{Read, Write};

fn main() {
    let mut number = false;
    let mut files: Vec<String> = Vec::new();
    for a in std::env::args().skip(1) {
        if a == "-n" {
            number = true;
        } else {
            files.push(a);
        }
    }
    let mut out: Vec<u8> = Vec::new();
    let mut counter = 0u64;
    let mut failed = false;
    let mut emit = |data: &[u8], out: &mut Vec<u8>| {
        if !number {
            out.extend_from_slice(data);
            return;
        }
        for line in data.split_inclusive(|&b| b == b'\n') {
            counter += 1;
            out.extend_from_slice(format!("{:6}\t", counter).as_bytes());
            out.extend_from_slice(line);
            if line.last() != Some(&b'\n') {
                out.push(b'\n');
            }
        }
    };
    if files.is_empty() {
        let mut d = Vec::new();
        std::io::stdin().read_to_end(&mut d).unwrap();
        emit(&d, &mut out);
    }
    for f in &files {
        match std::fs::read(f) {
            Ok(d) => emit(&d, &mut out),
            Err(_) => {
                out.extend_from_slice(format!("cat: {}: No such file\n", f).as_bytes());
                failed = true;
            }
        }
    }
    let so = std::io::stdout();
    so.lock().write_all(&out).unwrap();
    std::process::exit(if failed { 1 } else { 0 });
}

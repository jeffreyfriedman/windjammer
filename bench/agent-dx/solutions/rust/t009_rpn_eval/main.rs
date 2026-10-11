use std::io::Read;

fn eval(line: &str) -> Result<i64, &'static str> {
    let mut st: Vec<i64> = Vec::new();
    for tok in line.split(' ') {
        match tok {
            "+" | "-" | "*" | "/" => {
                if st.len() < 2 {
                    return Err("error: underflow");
                }
                let b = st.pop().unwrap();
                let a = st.pop().unwrap();
                st.push(match tok {
                    "+" => a + b,
                    "-" => a - b,
                    "*" => a * b,
                    _ => {
                        if b == 0 {
                            return Err("error: divzero");
                        }
                        a / b
                    }
                });
            }
            _ => match tok.parse::<i64>() {
                Ok(v) if !tok.starts_with('+') => st.push(v),
                _ => return Err("error: token"),
            },
        }
    }
    if st.len() == 1 {
        Ok(st[0])
    } else {
        Err("error: leftover")
    }
}

fn main() {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    for line in s.lines() {
        let l = line.trim_end_matches('\r');
        if l.trim().is_empty() {
            continue;
        }
        match eval(l) {
            Ok(v) => println!("{}", v),
            Err(e) => println!("{}", e),
        }
    }
}

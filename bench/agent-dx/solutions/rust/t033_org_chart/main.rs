use std::collections::HashMap;
use std::io::Read;

fn main() {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    let mut lines = s.lines();
    let n: usize = lines.next().unwrap().trim().parse().unwrap();
    let mut emp: HashMap<String, (String, i64)> = HashMap::new();
    for _ in 0..n {
        let l = lines.next().unwrap();
        let p: Vec<&str> = l.split_whitespace().collect();
        emp.insert(p[0].to_string(), (p[1].to_string(), p[2].parse().unwrap()));
    }
    for q in lines {
        let q = q.trim();
        if q.is_empty() {
            continue;
        }
        let out = (|| -> Result<i64, String> {
            let (m1, _) = emp.get(q).ok_or(format!("unknown {}", q))?;
            if m1 == "-" {
                return Err(format!("no manager {}", q));
            }
            let (m2, _) = emp.get(m1).ok_or(format!("unknown {}", m1))?;
            if m2 == "-" {
                return Err(format!("no skip-level {}", m1));
            }
            let (_, sal) = emp.get(m2).ok_or(format!("unknown {}", m2))?;
            Ok(*sal)
        })();
        match out {
            Ok(v) => println!("{}", v),
            Err(e) => println!("none: {}", e),
        }
    }
}

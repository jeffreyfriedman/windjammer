use std::cmp::Ordering;
use std::io::Read;

fn main() {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    let mut lines = s.lines();
    let spec: Vec<(usize, bool)> = lines
        .next()
        .unwrap_or("")
        .split(',')
        .filter(|x| !x.is_empty())
        .map(|k| {
            let (f, d) = k.split_once(':').unwrap();
            let idx = match f { "name" => 0, "age" => 1, _ => 2 };
            (idx, d == "desc")
        })
        .collect();
    let mut recs: Vec<Vec<&str>> = lines
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.split_whitespace().collect())
        .collect();
    recs.sort_by(|a, b| {
        for &(i, desc) in &spec {
            let o = if i == 1 {
                a[1].parse::<i64>().unwrap().cmp(&b[1].parse::<i64>().unwrap())
            } else {
                a[i].as_bytes().cmp(b[i].as_bytes())
            };
            let o = if desc { o.reverse() } else { o };
            if o != Ordering::Equal {
                return o;
            }
        }
        Ordering::Equal
    });
    for r in recs {
        println!("{}", r.join(" "));
    }
}

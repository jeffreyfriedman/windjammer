use std::io::Read;

struct Fib(i64, i64);
impl Iterator for Fib {
    type Item = i64;
    fn next(&mut self) -> Option<i64> {
        let r = self.0;
        self.0 = self.1;
        self.1 += r;
        Some(r)
    }
}

struct Primes(i64);
impl Iterator for Primes {
    type Item = i64;
    fn next(&mut self) -> Option<i64> {
        loop {
            self.0 += 1;
            let n = self.0;
            if n >= 2 && (2..).take_while(|d| d * d <= n).all(|d| n % d != 0) {
                return Some(n);
            }
        }
    }
}

fn seq(kind: &str) -> Option<Box<dyn Iterator<Item = i64>>> {
    Some(match kind {
        "fib" => Box::new(Fib(0, 1)),
        "squares" => Box::new((1i64..).map(|n| n * n)),
        "primes" => Box::new(Primes(1)),
        "evens" => Box::new((0i64..).map(|n| n * 2)),
        "triangular" => Box::new((1i64..).map(|n| n * (n + 1) / 2)),
        _ => return None,
    })
}

fn main() {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    for line in s.lines() {
        let p: Vec<&str> = line.split_whitespace().collect();
        if p.len() < 3 {
            continue;
        }
        let Some(it) = seq(p[0]) else { continue };
        let n: i64 = p[2].parse().unwrap();
        match p[1] {
            "take" => {
                let v: Vec<String> = it.take(n as usize).map(|x| x.to_string()).collect();
                println!("{}", v.join(" "));
            }
            "below" => {
                let v: Vec<String> = it.take_while(|&x| x < n).map(|x| x.to_string()).collect();
                println!("{}", v.join(" "));
            }
            "sum" => println!("{}", it.take(n as usize).sum::<i64>()),
            _ => {}
        }
    }
}

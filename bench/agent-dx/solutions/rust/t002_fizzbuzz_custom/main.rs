use std::io::Read;

fn main() {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    let v: Vec<u64> = s.split_whitespace().map(|x| x.parse().unwrap()).collect();
    let (n, a, b) = (v[0], v[1], v[2]);
    let mut out = String::new();
    for i in 1..=n {
        match (i % a == 0, i % b == 0) {
            (true, true) => out.push_str("FizzBuzz\n"),
            (true, false) => out.push_str("Fizz\n"),
            (false, true) => out.push_str("Buzz\n"),
            _ => out.push_str(&format!("{}\n", i)),
        }
    }
    print!("{}", out);
}

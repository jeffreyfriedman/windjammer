use std::io::Read;
use std::sync::{Arc, Mutex};
use std::thread;

fn divisors(n: u64) -> u64 {
    let mut c = 0;
    let mut d = 1;
    while d * d <= n {
        if n % d == 0 {
            c += if d * d == n { 1 } else { 2 };
        }
        d += 1;
    }
    c
}

fn main() {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    let mut it = s.split_whitespace();
    let w: usize = it.next().unwrap().parse().unwrap();
    let n: usize = it.next().unwrap().parse().unwrap();
    let vals: Arc<Vec<u64>> = Arc::new(it.take(n).map(|x| x.parse().unwrap()).collect());
    let next = Arc::new(Mutex::new(0usize));
    let results = Arc::new(Mutex::new(vec![0u64; vals.len()]));
    let mut handles = Vec::new();
    for _ in 0..w {
        let (vals, next, results) = (Arc::clone(&vals), Arc::clone(&next), Arc::clone(&results));
        handles.push(thread::spawn(move || loop {
            let i = {
                let mut g = next.lock().unwrap();
                let i = *g;
                *g += 1;
                i
            };
            if i >= vals.len() {
                break;
            }
            let r = divisors(vals[i]);
            results.lock().unwrap()[i] = r;
        }));
    }
    for h in handles {
        h.join().unwrap();
    }
    let res = results.lock().unwrap();
    for (v, r) in vals.iter().zip(res.iter()) {
        println!("{} {}", v, r);
    }
}

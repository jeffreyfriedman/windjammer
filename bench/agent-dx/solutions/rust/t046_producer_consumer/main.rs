use std::io::Read;
use std::sync::mpsc::sync_channel;
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    let v: Vec<usize> = s.split_whitespace().map(|x| x.parse().unwrap()).collect();
    let (p, k, c) = (v[0], v[1], v[2]);
    let (tx, rx) = sync_channel::<String>(4);
    let rx = Arc::new(Mutex::new(rx));
    let producers: Vec<_> = (0..p)
        .map(|id| {
            let tx = tx.clone();
            thread::spawn(move || {
                for j in 1..=k {
                    tx.send(format!("p{}-{}", id, j)).unwrap();
                }
            })
        })
        .collect();
    drop(tx);
    let consumers: Vec<_> = (0..c)
        .map(|_| {
            let rx = Arc::clone(&rx);
            thread::spawn(move || {
                let mut got: Vec<String> = Vec::new();
                loop {
                    let item = rx.lock().unwrap().recv();
                    match item {
                        Ok(x) => got.push(x),
                        Err(_) => break,
                    }
                }
                got
            })
        })
        .collect();
    for h in producers {
        h.join().unwrap();
    }
    let mut all: Vec<String> = Vec::new();
    let mut consumed = 0;
    for h in consumers {
        let got = h.join().unwrap();
        consumed += got.len();
        all.extend(got);
    }
    let chars: usize = all.iter().map(|x| x.len()).sum();
    all.sort();
    println!("items: {}", p * k);
    println!("chars: {}", chars);
    for x in all.iter().take(3) {
        println!("first: {}", x);
    }
    println!("consumers_total: {}", consumed);
}

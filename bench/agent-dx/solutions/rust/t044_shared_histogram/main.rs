use std::io::Read;
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    let mut it = s.split_whitespace();
    let w: usize = it.next().unwrap().parse().unwrap();
    let n: usize = it.next().unwrap().parse().unwrap();
    let vals: Arc<Vec<usize>> = Arc::new(it.take(n).map(|x| x.parse().unwrap()).collect());
    let hist = Arc::new(Mutex::new([0u64; 10]));
    let mut handles = Vec::new();
    for id in 0..w {
        let vals = Arc::clone(&vals);
        let hist = Arc::clone(&hist);
        handles.push(thread::spawn(move || {
            let mut count = 0u64;
            let mut i = id;
            while i < vals.len() {
                hist.lock().unwrap()[vals[i]] += 1;
                count += 1;
                i += w;
            }
            count
        }));
    }
    let counts: Vec<u64> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    let h = hist.lock().unwrap();
    let strs: Vec<String> = h.iter().map(|c| c.to_string()).collect();
    println!("hist: {}", strs.join(" "));
    for (i, c) in counts.iter().enumerate() {
        println!("worker {}: {}", i, c);
    }
    let checksum: u64 = h.iter().enumerate().map(|(d, c)| d as u64 * c).sum();
    println!("checksum: {}", checksum);
}

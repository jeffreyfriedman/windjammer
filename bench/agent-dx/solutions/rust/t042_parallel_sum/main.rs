use std::io::Read;
use std::sync::mpsc;
use std::thread;

fn main() {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    let mut it = s.split_whitespace();
    let w: usize = it.next().unwrap().parse().unwrap();
    let n: usize = it.next().unwrap().parse().unwrap();
    let vals: Vec<i64> = it.take(n).map(|x| x.parse().unwrap()).collect();
    let (q, r) = (n / w, n % w);
    let (tx, rx) = mpsc::channel();
    let mut start = 0;
    let mut handles = Vec::new();
    for i in 0..w {
        let len = if i < r { q + 1 } else { q };
        let chunk: Vec<i64> = vals[start..start + len].to_vec();
        start += len;
        let tx = tx.clone();
        handles.push(thread::spawn(move || {
            tx.send((i, chunk.iter().sum::<i64>())).unwrap();
        }));
    }
    drop(tx);
    let mut sums = vec![0i64; w];
    for (i, v) in rx {
        sums[i] = v;
    }
    for h in handles {
        h.join().unwrap();
    }
    for (i, v) in sums.iter().enumerate() {
        println!("chunk {}: {}", i, v);
    }
    println!("total: {}", sums.iter().sum::<i64>());
}

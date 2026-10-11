use std::collections::{BTreeMap, HashMap};
use std::io::Read;

struct Task {
    duration: u64,
    deps: Vec<i64>,
}

fn main() {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).unwrap();
    let mut lines = s.lines();
    let mut hd = lines.next().unwrap_or("0 0").split_whitespace();
    let w: usize = hd.next().unwrap().parse().unwrap();
    let n: usize = hd.next().unwrap().parse().unwrap();
    let mut tasks: BTreeMap<i64, Task> = BTreeMap::new();
    for _ in 0..n {
        let p: Vec<i64> = lines.next().unwrap().split_whitespace().map(|x| x.parse().unwrap()).collect();
        tasks.insert(p[0], Task { duration: p[1] as u64, deps: p[2..].to_vec() });
    }
    let mut times: HashMap<i64, (u64, u64)> = HashMap::new(); // id -> (start, end)
    let mut running: Vec<(u64, i64)> = Vec::new(); // (end, id)
    let mut done: HashMap<i64, u64> = HashMap::new();
    let mut t = 0u64;
    while times.len() < n {
        let (fin, still): (Vec<_>, Vec<_>) = running.iter().partition(|(e, _)| *e <= t);
        for (e, id) in fin {
            done.insert(id, e);
        }
        running = still;
        for (id, task) in &tasks {
            if running.len() >= w {
                break;
            }
            if times.contains_key(id) {
                continue;
            }
            if task.deps.iter().all(|d| done.get(d).map_or(false, |&e| e <= t)) {
                times.insert(*id, (t, t + task.duration));
                running.push((t + task.duration, *id));
            }
        }
        match running.iter().map(|r| r.0).min() {
            Some(m) => t = m,
            None => break,
        }
    }
    let mut makespan = 0;
    for id in tasks.keys() {
        let (a, b) = times[id];
        println!("{} {} {}", id, a, b);
        makespan = makespan.max(b);
    }
    println!("makespan {}", makespan);
}

//! Tiny bounded-parallel map (std threads only).

use std::sync::{Arc, Mutex};

pub fn map<I: Send + 'static, O: Send + 'static, F>(items: Vec<I>, workers: usize, f: F) -> Vec<O>
where
    F: Fn(I) -> O + Send + Sync + 'static,
{
    let queue = Arc::new(Mutex::new(items.into_iter().enumerate().collect::<Vec<_>>()));
    let results = Arc::new(Mutex::new(Vec::new()));
    let f = Arc::new(f);
    let handles: Vec<_> = (0..workers.max(1))
        .map(|_| {
            let (q, r, f) = (queue.clone(), results.clone(), f.clone());
            std::thread::spawn(move || loop {
                let job = q.lock().unwrap().pop();
                let Some((idx, item)) = job else { break };
                let out = f(item);
                r.lock().unwrap().push((idx, out));
            })
        })
        .collect();
    for h in handles {
        h.join().expect("worker panicked");
    }
    let mut v = Arc::try_unwrap(results).ok().unwrap().into_inner().unwrap();
    v.sort_by_key(|(i, _)| *i);
    v.into_iter().map(|(_, o)| o).collect()
}

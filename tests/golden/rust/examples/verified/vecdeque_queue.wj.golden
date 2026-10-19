use std::collections::VecDeque;
fn main() {
    let mut q: VecDeque<i64> = VecDeque::new();
    q.push_back(1_i64);
    q.push_back(2_i64);
    q.push_back(3_i64);
    q.push_front(0_i64);
    println!("{:?}", q.get(1).copied());
    println!("{:?}", q.pop_front());
    println!("{}", q.len());
    q.pop_front();
    q.push_front(0_i64);
    let v: Vec<i64> = q.iter().cloned().collect::<Vec<i64>>();
    println!("{:?}", v);
}


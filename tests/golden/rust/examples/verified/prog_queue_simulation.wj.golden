use std::collections::VecDeque;
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]

struct Customer {
    id: String,
    arrival: i64,
}

fn main() {
    let mut q: VecDeque<Customer> = VecDeque::new();
    q.push_back(Customer { id: String::from("c1"), arrival: 1_i64 });
    q.push_back(Customer { id: String::from("c2"), arrival: 2_i64 });
    q.push_back(Customer { id: String::from("c3"), arrival: 3_i64 });
    let mut clock = 1_i32;
    loop {
        if let Some(c) = q.pop_front() {
            let _ = println!("t={} serve {} (waited {})", clock, c.id, clock as i64 - c.arrival);
            clock += 2;
        } else {
            break;
        }
    }
}


#[derive(Debug, Clone, Default)]

struct Bus {
    handlers: Vec<fn(&str) -> String>,
}

fn main() {
    let mut bus = Bus { handlers: vec![] };
    { bus.handlers.push(|e| format!("log: {}", e)) };
    { bus.handlers.push(|e| format!("log: {} 3", e)) };
    let events = vec!["started".to_string(), "clicked".to_string()];
    let mut handled = 0_i32;
    for (i, h) in bus.handlers.iter().enumerate() {
        println!("{}", h(events[i as usize].clone()));
        handled += 1;
    }
    println!("total handled: {}", handled);
}


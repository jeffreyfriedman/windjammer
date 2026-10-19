#[inline]
fn takes_ownership(v: &Vec<i64>) {
    println!("Received vector with {} elements", v.len());
}

#[inline]
fn process_and_drop(v: &Vec<i64>) {
    println!("Processing {} elements", v.len());
}

fn main() {
    let data = vec![1_i64, 2_i64, 3_i64, 4_i64, 5_i64];
    takes_ownership(&data);
    println!("Data length: {}", data.len());
    let message: String = String::from("Hello, Windjammer!");
    send_message(&message);
    log_message(&message);
    println!("Message: {}", message);
    let temp = vec![10_i64, 20_i64, 30_i64];
    process_and_drop(&temp);
    println!("Auto-clone test complete!");
}

#[inline]
fn send_message(msg: &str) {
    println!("Sending: {}", msg);
}

#[inline]
fn log_message(msg: &str) {
    println!("Logging: {}", msg);
}


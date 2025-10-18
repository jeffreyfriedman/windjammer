use smallvec::{SmallVec, smallvec};

const MAX_USERS: i64 = 1000;
static CACHE_SIZE: i64 = /* expression */;
const DEFAULT_TIMEOUT: f64 = 30.0;

#[inline]
fn get_fibonacci_sequence(count: i64) -> vec<i64> {
    if count <= 3 {
        vec![1, 1, 2]
    } else {
        vec![1, 1, 2, 3, 5, 8]
    }
}

#[inline]
fn format_username(name: &String, capitalize: bool) -> String {
    if capitalize {
        name.to_uppercase()
    } else {
        name
    }
}

#[inline]
fn process_data(numbers: &vec<i64>) -> i64 {
    let sum = numbers.iter().sum();
    sum
}

#[inline]
fn build_message(user: &String, action: &String) -> String {
    let mut msg = "User: ";
    msg.push_str(user);
    msg.push_str(" performed: ");
    msg.push_str(action);
    msg
}

#[inline]
fn get_user_count(cache: &HashMap<String, vec<i64>>) -> i64 {
    cache.len()
}

#[inline]
fn is_valid_user_id(id: i64) -> bool {
    id > 0 && id <= MAX_USERS
}

#[inline]
fn increment_counter(count: i64) -> i64 {
    let mut result = count;
    result += 1;
    result
}

#[inline]
fn calculate_buffer_size() -> i64 {
    MAX_USERS * 4 + 128
}

fn main() {
    println!("=== Windjammer Compiler Optimization Demo ===");
    println!("");
    println!("Phase 7 - Const/Static:");
    println!("  MAX_USERS = {}", MAX_USERS);
    println!("  CACHE_SIZE = {}", CACHE_SIZE);
    println!("");
    println!("Phase 8 - SmallVec:");
    let fib = get_fibonacci_sequence(3);
    println!("  Fibonacci (stack allocated): {:?}", fib);
    println!("");
    println!("Phase 9 - Cow (Clone-on-Write):");
    let name1 = format_username(&"alice", false);
    let name2 = format_username(&"bob", true);
    println!("  Normal: {}", name1);
    println!("  Uppercase: {}", name2);
    println!("");
    println!("Phase 2 - Clone Elimination:");
    let nums: SmallVec<[_; 8]> = smallvec![1, 2, 3, 4, 5];
    let total = process_data(&nums);
    println!("  Sum: {}", total);
    println!("");
    println!("Phase 4 - String Capacity:");
    let msg = build_message(&"Alice", &"login");
    println!("  Message: {}", msg);
    println!("");
    println!("Phase 1 - Inline Hints:");
    println!("  Is valid(500)? {}", is_valid_user_id(500));
    println!("  Is valid(-1)? {}", is_valid_user_id(-1));
    println!("");
    println!("Phase 5 - Compound Assignments:");
    println!("  Increment 41: {}", increment_counter(41));
    println!("");
    println!("Phase 6 - Constant Folding:");
    println!("  Buffer size: {}", calculate_buffer_size());
    println!("");
    println!("All optimizations applied automatically! 🚀")
}


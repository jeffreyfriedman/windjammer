#[inline]
fn find_in_vec(items: &Vec<String>, target: &str) -> bool {
    let mut i: usize = 0_usize;
    while i < items.len() {
        if items[i] == target {
            return true;
        }
        i += 1;
    }
    false
}

fn main() {
    let items = vec!["hello".to_string(), "world".to_string()];
    let result = find_in_vec(&items, "hello");
    println!("Result: {}", result);
}


#[inline]
fn get_value(opt: Option<i32>) -> i32 {
    let value = match opt {
        Some(x) => x,
        None => {
            return;
        },
    };
    value * 2_i32
}

#[inline]
fn process_option(opt: Option<String>) -> String {
    match opt {
        Some(s) => s,
        None => {
            return;
        },
    }
}

#[inline]
fn complex_match(x: i32) -> i32 {
    match x {
        0 => {
            return;
        },
        1 => 10_i32,
        2 => 20_i32,
        _ => 30_i32,
    }
}

fn main() {
    let result1 = get_value(Some(5_i32));
    assert!(result1 == 10_i32, "Should double the value");
    println!("✅ Return in match arms works!");
}


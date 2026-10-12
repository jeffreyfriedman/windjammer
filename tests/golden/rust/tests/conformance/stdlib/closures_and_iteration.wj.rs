#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]

struct Counter {
    value: i64,
}









impl Counter {
#[inline]
fn new() -> Counter {
        Counter { value: 0_i64 }
}
#[inline]
fn next(&mut self) -> i64 {
        self.value += 1;
        self.value
}
}

#[inline]
fn test_closure_capture() {
    let prefix: String = String::from("Hello");
    let name: String = String::from("World");
    let greeting = format!("{}{}{}{}", prefix, ", ", name, "!");
    println!("[closure_capture] greeting: {}", greeting);
    let factor = 3_i32;
    let x = 5_i32;
    let result = x * factor;
    println!("[closure_capture] multiplied: {}", result);
}

fn test_closure_map() {
    let numbers = vec![1_i32, 2_i32, 3_i32, 4_i32, 5_i32];
    let mut doubled: Vec<i64> = vec![];
    for n in numbers {
        doubled.push(n * 2_i64);
    }
    let mut result = String::new();
    for d in doubled {
        if !result.is_empty() {
            { use std::fmt::Write as _; write!(result, "{}", " ").expect("a formatting trait implementation returned an error"); };
        }
        result = result + &format!("{}", d);
    }
    println!("[closure_map] doubled: {}", result);
}

fn test_closure_filter() {
    let numbers = vec![1_i32, 2_i32, 3_i32, 4_i32, 5_i32, 6_i32, 7_i32, 8_i32, 9_i32, 10_i32];
    let mut evens: Vec<i64> = vec![];
    for n in numbers {
        if n % 2_i64 == 0_i64 {
            let _ = evens.push(n);
        }
    }
    let mut result = String::new();
    for e in evens {
        if !result.is_empty() {
            { use std::fmt::Write as _; write!(result, "{}", " ").expect("a formatting trait implementation returned an error"); };
        }
        result = result + &format!("{}", e);
    }
    println!("[closure_filter] evens: {}", result);
}

#[inline]
fn test_closure_fold() {
    let numbers = vec![1_i32, 2_i32, 3_i32, 4_i32, 5_i32];
    let mut sum = 0_i32;
    for n in numbers {
        sum += n as i32;
    }
    println!("[closure_fold] sum: {}", sum);
}

fn test_closure_chain() {
    let numbers = vec![1_i32, 2_i32, 3_i32, 4_i32, 5_i32];
    let mut result_vec: Vec<i64> = vec![];
    for n in numbers {
        if n % 2_i64 == 0_i64 {
            let _ = result_vec.push(n * n);
        }
    }
    let mut result = String::new();
    for r in result_vec {
        if !result.is_empty() {
            { use std::fmt::Write as _; write!(result, "{}", " ").expect("a formatting trait implementation returned an error"); };
        }
        result = result + &format!("{}", r);
    }
    println!("[closure_chain] result: {}", result);
}

#[inline]
fn test_closure_counter() {
    let mut counter = Counter::new();
    println!("[closure_counter] count: {}", counter.next());
    println!("[closure_counter] count: {}", counter.next());
    println!("[closure_counter] count: {}", counter.next());
}

#[inline]
fn test_enumerate() {
    let fruits = vec!["apple".to_string(), "banana".to_string(), "cherry".to_string()];
    let mut result = String::new();
    let mut idx: i32 = 0_i32;
    for fruit in fruits {
        if idx > 0_i32 {
            { use std::fmt::Write as _; write!(result, "{}", " ").expect("a formatting trait implementation returned an error"); };
        }
        result = result + &format!("{}:{}", idx, fruit);
        idx += 1;
    }
    println!("[closure_enumerate] {}", result);
}

fn test_nested_iter() {
    let matrix = vec![vec![1_i32, 2_i32, 3_i32], vec![4_i32, 5_i32, 6_i32]];
    let mut flat: Vec<i64> = vec![];
    for row in matrix {
        for item in row.iter().copied() {
            flat.push(item);
        }
    }
    let mut result = String::new();
    for f in flat {
        if !result.is_empty() {
            { use std::fmt::Write as _; write!(result, "{}", " ").expect("a formatting trait implementation returned an error"); };
        }
        result = result + &format!("{}", f);
    }
    println!("[closure_nested_iter] flat: {}", result);
}

fn main() {
    test_closure_capture();
    test_closure_map();
    test_closure_filter();
    test_closure_fold();
    test_closure_chain();
    test_closure_counter();
    test_enumerate();
    test_nested_iter();
    println!("[closure_all] PASSED");
}


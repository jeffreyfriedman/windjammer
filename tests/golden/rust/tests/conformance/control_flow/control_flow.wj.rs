#[inline]
fn test_if_else() {
    let x = 10_i32;
    if x > 5_i32 {
        println!("[if_true] branch: yes");
    } else {
        println!("[if_true] branch: no");
    }
    let y = 2_i32;
    if y > 5_i32 {
        println!("[if_false] branch: yes");
    } else {
        println!("[if_false] branch: no");
    }
}

#[inline]
fn sign(x: i64) -> String {
    if x > 0_i64 {
        String::from("positive")
    } else {
        if x < 0_i64 {
            String::from("negative")
        } else {
            String::from("zero")
        }
    }
}

#[inline]
fn test_if_expression() {
    println!("[if_expr] sign of 5: {}", sign(5_i64));
    println!("[if_expr] sign of -3: {}", sign(-3_i64));
    println!("[if_expr] sign of 0: {}", sign(0_i64));
}

#[inline]
fn categorize(x: i64) -> String {
    if x < 10_i64 {
        String::from("small")
    } else {
        if x < 100_i64 {
            String::from("medium")
        } else {
            String::from("large")
        }
    }
}

#[inline]
fn test_if_chain() {
    println!("[if_chain] {}", categorize(5_i64));
    println!("[if_chain] {}", categorize(50_i64));
    println!("[if_chain] {}", categorize(500_i64));
}

fn test_while() {
    let mut count: i32 = 0_i32;
    while count < 5_i32 {
        count += 1;
    }
    println!("[while] count to 5: {}", count);
    let mut sum = 0_i64;
    let mut i: i32 = 1_i32;
    while i <= 5_i32 {
        sum += i as i64;
        i += 1;
    }
    println!("[while] sum 1..5: {}", sum);
}

fn test_for_range() {
    let mut sum = 0_i32;
    for i in 0_i32..5_i32 {
        sum += i;
    }
    println!("[for_range] sum 0..5: {}", sum);
    let mut sum2 = 0_i32;
    for i in 1_i32..=5_i32 {
        sum2 += i;
    }
    println!("[for_range] sum 1..=5: {}", sum2);
}

#[inline]
fn test_for_vec() {
    let fruits = vec!["apple".to_string(), "banana".to_string(), "cherry".to_string()];
    let mut result = String::new();
    for fruit in fruits {
        if !result.is_empty() {
            { use std::fmt::Write as _; write!(result, "{}", " ").expect("a formatting trait implementation returned an error"); };
        }
        result = result + &fruit;
    }
    println!("[for_vec] items: {}", result);
}

#[inline]
fn test_for_index() {
    let letters = vec!["a".to_string(), "b".to_string(), "c".to_string()];
    let mut result = String::new();
    let mut idx: i32 = 0_i32;
    for letter in letters {
        if idx > 0_i32 {
            { use std::fmt::Write as _; write!(result, "{}", " ").expect("a formatting trait implementation returned an error"); };
        }
        result = result + &format!("{}:{}", idx, letter);
        idx += 1;
    }
    println!("[for_index] {}", result);
}

#[inline]
fn test_loop_break() {
    let mut i = 0_i32;
    loop {
        if i >= 3_i32 {
            break;
        }
        i += 1;
    }
    println!("[loop_break] loop exited at: {}", i);
}

#[inline]
fn test_continue() {
    let mut result = String::new();
    for i in 0_i32..10_i32 {
        if i % 2_i32 == 0_i32 {
            continue;
        }
        if !result.is_empty() {
            { use std::fmt::Write as _; write!(result, "{}", " ").expect("a formatting trait implementation returned an error"); };
        }
        result = result + &format!("{}", i);
    }
    println!("[continue] odd numbers: {}", result);
}

#[inline]
fn test_nested() {
    let mut result = String::new();
    for i in 0_i32..3_i32 {
        for j in 0_i32..2_i32 {
            if !result.is_empty() {
                { use std::fmt::Write as _; write!(result, "{}", " ").expect("a formatting trait implementation returned an error"); };
            }
            result = result + &format!("({},{})", i, j);
        }
    }
    println!("[nested] {}", result);
}

#[inline]
fn test_short_circuit() {
    println!("[shortcircuit] true && true = {}", true);
    println!("[shortcircuit] true && false = {}", false);
    println!("[shortcircuit] false && true = {}", false);
    println!("[shortcircuit] true || false = {}", true);
    println!("[shortcircuit] false || true = {}", true);
    println!("[shortcircuit] false || false = {}", false);
}

fn main() {
    test_if_else();
    test_if_expression();
    test_if_chain();
    test_while();
    test_for_range();
    test_for_vec();
    test_for_index();
    test_loop_break();
    test_continue();
    test_nested();
    test_short_circuit();
    println!("[control_all] PASSED");
}


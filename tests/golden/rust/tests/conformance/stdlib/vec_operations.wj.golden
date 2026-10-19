#[inline]
fn test_vec_create() {
    let empty: Vec<i64> = vec![];
    println!("[vec_create] empty len={}", empty.len());
    let v = vec![10_i64, 20_i64, 30_i64];
    println!("[vec_create] with elements len={}", v.len());
    let mut elems = String::new();
    for item in v {
        if !elems.is_empty() {
            { use std::fmt::Write as _; write!(elems, "{}", " ").expect("a formatting trait implementation returned an error"); };
        }
        elems = elems + &format!("{}", item);
    }
    println!("[vec_create] elements: {}", elems);
}

#[inline]
fn test_vec_push() {
    let mut v = vec![10_i64, 20_i64, 30_i64];
    v.push(40_i64);
    println!("[vec_push] after push: len={}", v.len());
    println!("[vec_push] last: {}", v[3]);
}

#[inline]
fn test_vec_pop() {
    let mut v = vec![10_i64, 20_i64, 30_i64];
    let popped = v.pop();
    match popped {
        Some(val) => println!("[vec_pop] popped: {}", val),
        None => println!("[vec_pop] empty!"),
    }
    println!("[vec_pop] after pop: len={}", v.len());
}

#[inline]
fn test_vec_index() {
    let mut v = vec![10_i64, 20_i64, 30_i64];
    println!("[vec_index] v[0]={}, v[1]={}, v[2]={}", v[0], v[1], v[2]);
}

#[inline]
fn test_vec_iterate() {
    let v = vec![10_i64, 20_i64, 30_i64];
    let mut sum = 0_i64;
    for item in v {
        sum += item;
    }
    println!("[vec_iterate] sum={}", sum);
}

#[inline]
fn test_vec_empty() {
    let mut v: Vec<i64> = vec![];
    println!("[vec_empty] is_empty before: {}", v.is_empty());
    v.push(1_i64);
    println!("[vec_empty] is_empty after push: {}", v.is_empty());
}

#[inline]
fn test_vec_nested() {
    let mut matrix = vec![vec![1_i32, 2_i32, 3_i32], vec![4_i32, 5_i32, 6_i32], vec![7_i32, 8_i32, 9_i32]];
    println!("[vec_nested] matrix[0][0]={}, matrix[1][1]={}, matrix[2][2]={}", matrix[0][0], matrix[1][1], matrix[2][2]);
}

#[inline]
fn test_vec_build() {
    let mut squares: Vec<i64> = vec![];
    for i in 0_i32..5_i32 {
        squares.push((i * i) as i64);
    }
    let mut result = String::new();
    for s in squares {
        if !result.is_empty() {
            { use std::fmt::Write as _; write!(result, "{}", " ").expect("a formatting trait implementation returned an error"); };
        }
        result = result + &format!("{}", s);
    }
    println!("[vec_build] built: {}", result);
}

fn main() {
    test_vec_create();
    test_vec_push();
    test_vec_pop();
    test_vec_index();
    test_vec_iterate();
    test_vec_empty();
    test_vec_nested();
    test_vec_build();
    println!("[vec_all] PASSED");
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]

struct Point {
    x: i32,
    y: i32,
}










#[inline]
fn test_tuple_iteration() -> i32 {
    let pairs = vec![(1_i32, 2_i32), (3_i32, 4_i32), (5_i32, 6_i32)];
    let mut sum: i32 = 0_i32;
    for (a, b) in pairs {
        sum = sum + a as i32 + b;
    }
    sum
}

#[inline]
fn test_struct_iteration() -> i32 {
    let points = vec![Point { x: 1_i32, y: 2_i32 }, Point { x: 3_i32, y: 4_i32 }, Point { x: 5_i32, y: 6_i32 }];
    let mut sum: i32 = 0_i32;
    for Point { x, y } in points {
        sum = sum + x + y;
    }
    sum
}

#[inline]
fn test_wildcard_iteration() -> i32 {
    let pairs = vec![(1_i32, 100_i32), (2_i32, 200_i32), (3_i32, 300_i32)];
    let mut sum: i32 = 0_i32;
    for (a, _) in pairs {
        sum += a as i32;
    }
    sum
}

#[inline]
fn test_nested_iteration() -> i32 {
    let nested = vec![((1_i32, 2_i32), (3_i32, 4_i32)), ((5_i32, 6_i32), (7_i32, 8_i32))];
    let mut sum: i32 = 0_i32;
    for ((a, b), (c, d)) in nested {
        sum = sum + a + b + c + d;
    }
    sum
}

fn main() {
    let _r1 = test_tuple_iteration();
    let _r2 = test_struct_iteration();
    let _r3 = test_wildcard_iteration();
    let _r4 = test_nested_iteration();
    println!("All for loop pattern tests passed!");
}


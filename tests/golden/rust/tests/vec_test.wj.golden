use windjammer_runtime::test::*;
#[test]

fn test_vec_creation() {
    let v = vec![1_i64, 2_i64, 3_i64];
    assert!(v.len() == 3_usize);
}

#[test]

fn test_vec_push() {
    let mut v = vec![1_i64, 2_i64];
    v.push(3_i64);
    assert!(v.len() == 3_usize);
}

#[test]

fn test_vec_access() {
    let v = vec![10_i64, 20_i64, 30_i64];
    assert!(v[0] == 10_i64);
    assert!(v[1] == 20_i64);
    assert!(v[2] == 30_i64);
}

#[test]

fn test_vec_empty() {
    let v: Vec<i64> = vec![];
    assert!(v.is_empty());
}

#[test]

fn test_vec_iteration() {
    let v = vec![1_i64, 2_i64, 3_i64];
    let mut sum = 0_i64;
    for item in v {
        sum += item;
    }
    assert!(sum == 6_i64);
}

#[test]

fn test_vec_strings() {
    let v = vec!["hello".to_string(), "world".to_string()];
    assert!(v.len() == 2_usize);
    assert!(v[0] == "hello");
    assert!(v[1] == "world");
}


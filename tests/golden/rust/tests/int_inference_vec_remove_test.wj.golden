use windjammer_runtime::test::*;
#[test]

fn test_vec_remove_with_usize() {
    let mut items: Vec<i32> = Vec::new();
    items.push(1_i32);
    items.push(2_i32);
    items.push(3_i32);
    let index: usize = 1_usize;
    items.remove(index);
    items.remove(0_usize);
}

#[test]

fn test_vec_remove_with_loop_index() {
    let mut items: Vec<String> = Vec::new();
    items.push(String::from("a"));
    items.push(String::from("b"));
    items.push(String::from("c"));
    for i in 0_usize..items.len() {
        if i == 1_usize {
            let _ = items.remove(i);
            break;
        }
    }
}

#[test]

fn test_vec_remove_from_back() {
    let mut items: Vec<f32> = Vec::new();
    items.push(1.0_f32);
    items.push(2.0_f32);
    let last_idx = items.len() - 1_usize;
    items.remove(last_idx);
}


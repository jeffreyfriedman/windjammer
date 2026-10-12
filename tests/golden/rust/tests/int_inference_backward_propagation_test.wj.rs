use windjammer_runtime::test::*;
#[test]

fn test_array_index_backward_propagation() {
    let items: Vec<i32> = Vec::new();
    let idx = 0_i32;
    let _value = items[(idx as usize)];
}

#[test]

fn test_loop_index_backward_propagation() {
    let items: Vec<String> = Vec::new();
    let mut i: usize = 0_usize;
    while i < items.len() {
        let _value = items[i].to_string();
        i += 1;
    }
}

#[test]

fn test_function_parameter_backward_propagation() {
    let mut items: Vec<f32> = Vec::new();
    let idx = 5_i32;
    items.remove(idx as usize);
}


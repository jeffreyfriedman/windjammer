use windjammer_runtime::test::*;
#[test]

fn test_i32_loop_counter_with_array() {
    let items: Vec<String> = Vec::new();
    let mut i: i32 = 0_i32;
    while i < 10_i32 {
        if i < (items.len() as i32) {
            let __value = items[i as usize].to_string();
        }
        i += 1;
    }
}

#[test]

fn test_i32_capacity_with_array_access() {
    let slots: Vec<Option<i32>> = Vec::new();
    let capacity: i32 = 10_i32;
    let mut i: i32 = 0_i32;
    while i < capacity {
        let __slot = slots[i as usize];
        i += 1;
    }
}

#[inline]
fn get_slot_helper(slots: &Vec<Option<i32>>, index: i32) -> Option<i32> {
    if index < 0_i32 || index >= slots.len() as i32 {
        return None as i32;
    }
    slots[index as usize]
}

#[test]

fn test_i32_parameter_as_array_index() {
    let slots: Vec<Option<i32>> = Vec::new();
    get_slot_helper(&slots, 5_i32);
}


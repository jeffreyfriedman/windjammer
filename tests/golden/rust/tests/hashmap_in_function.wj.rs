use std::collections::HashMap;
#[inline]
fn test_hashmap() -> Vec<u32> {
    let mut came_from: HashMap<u32, u32> = HashMap::new();
    let mut g_score: HashMap<u32, f32> = HashMap::new();
    came_from.insert(1_u32, 2_u32);
    g_score.insert(1_u32, 1.0_f32);
    Vec::<u32>::new()
}

fn main() {
    let _result: Vec<u32> = test_hashmap();
    println!("✅ HashMap in functions works!");
}


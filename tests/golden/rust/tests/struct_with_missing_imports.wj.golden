use std::collections::HashMap;
#[derive(Debug, Clone)]
#[repr(C)]
pub struct AnimationController {
    pub animations: HashMap<String, Animation>,
    pub current_animation: Option<String>,
}

impl AnimationController {
#[inline]
pub fn new() -> AnimationController {
        AnimationController { animations: HashMap::new(), current_animation: None }
}
}

fn main() {
    let _ctrl = AnimationController::new();
    println!("This should compile (or give type error, not parse error)");
}


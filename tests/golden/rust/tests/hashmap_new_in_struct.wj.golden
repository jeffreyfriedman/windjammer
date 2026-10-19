use std::collections::HashMap;
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]

struct Animation {
    name: String,
}

#[derive(Debug, Clone, PartialEq)]

struct Controller {
    animations: HashMap<String, Animation>,
    current: Option<String>,
    index: usize,
    time: f32,
    playing: bool,
    looping: bool,
    speed: f32,
}

impl Controller {
#[inline]
pub fn new() -> Controller {
        Controller { animations: HashMap::new(), current: None, index: 0_usize, time: 0.0_f32, playing: false, looping: true, speed: 1.0_f32 }
}
}

fn main() {
    let _ctrl = Controller::new();
    println!("✅ HashMap::new() in struct literals works!");
}


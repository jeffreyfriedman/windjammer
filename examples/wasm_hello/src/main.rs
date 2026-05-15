use wasm_bindgen::prelude::*;

#[wasm_bindgen]
struct Counter {
    value: i32,
}

#[wasm_bindgen]
impl Counter {
pub fn new() -> Counter {
        Counter { value: 0 }
}
pub fn increment(&mut self) {
        self.value = self.value + 1;
}
pub fn get_value(&self) -> i32 {
        self.value
}
}

fn greet(name: &String) -> String {
    format!("Hello, {}! 👋 from Windjammer", name)
}

fn add(a: i32, b: i32) -> i32 {
    a + b
}


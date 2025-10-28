pub mod log {
pub fn init() {
    println!("Logger initialized")
}

pub fn error(message: &str) {
    println!("[ERROR] {}", message)
}

pub fn warn(message: &str) {
    println!("[WARN] {}", message)
}

pub fn info(message: &str) {
    println!("[INFO] {}", message)
}

pub fn debug(message: &str) {
    println!("[DEBUG] {}", message)
}

pub fn trace(message: &str) {
    println!("[TRACE] {}", message)
}


}


use log::*;


fn main() {
    println!("Testing std/log module...");
    init();
    trace("This is a trace message");
    debug("This is a debug message");
    info("This is an info message");
    warn("This is a warning message");
    error("This is an error message");
    println!("std/log works! ✓")
}


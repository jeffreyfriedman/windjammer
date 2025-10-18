#[inline]
fn init() {
}

#[inline]
fn init_with_level(level: &String) {
}

#[inline]
fn trace(message: &String) {
    println!("[TRACE] {}", message)
}

#[inline]
fn debug(message: &String) {
    println!("[DEBUG] {}", message)
}

#[inline]
fn info(message: &String) {
    println!("[INFO] {}", message)
}

#[inline]
fn warn(message: &String) {
    println!("[WARN] {}", message)
}

#[inline]
fn error(message: &String) {
    println!("[ERROR] {}", message)
}

#[inline]
fn trace_with(message: &String, key: &String, value: &String) {
    println!("[TRACE] {} - {}: {}", message, key, value)
}

#[inline]
fn debug_with(message: &String, key: &String, value: &String) {
    println!("[DEBUG] {} - {}: {}", message, key, value)
}

#[inline]
fn info_with(message: &String, key: &String, value: &String) {
    println!("[INFO] {} - {}: {}", message, key, value)
}

#[inline]
fn warn_with(message: &String, key: &String, value: &String) {
    println!("[WARN] {} - {}: {}", message, key, value)
}

#[inline]
fn error_with(message: &String, key: &String, value: &String) {
    println!("[ERROR] {} - {}: {}", message, key, value)
}

#[inline]
fn trace_enabled() -> bool {
    false
}

#[inline]
fn debug_enabled() -> bool {
    false
}

#[inline]
fn info_enabled() -> bool {
    true
}

#[inline]
fn warn_enabled() -> bool {
    true
}

#[inline]
fn error_enabled() -> bool {
    true
}


extern "C" {
    pub fn test_ffi_function(x: i32, y: i32) -> bool;
}

#[inline]
fn check_collision(tile_x: i32, tile_y: i32) -> bool {
    (unsafe { test_ffi_function(tile_x, tile_y) })
}

fn main() {
    println!("✅ FFI function declarations work!");
}


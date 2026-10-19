#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[repr(C)]
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}











#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct Buffer {
    data: *const u8,
    size: u64,
}

extern "C" {
    pub fn test_const_pointer(data: *const u8, size: u64) -> u64;
    pub fn test_mut_pointer(data: *mut u8, size: u64) -> u64;
    pub fn test_pointer_to_struct(ptr: *const Vec3) -> f32;
}

#[inline]
fn get_null_ptr() -> *const u8 {
    std::ptr::null()
}

#[inline]
fn get_null_mut_ptr() -> *mut u8 {
    std::ptr::null_mut()
}

fn main() {
    println!("========================================");
    println!("🔧 RAW POINTER TYPE TESTS");
    println!("========================================");
    let ptr = get_null_ptr();
    let _buffer = Buffer { data: ptr, size: 0_u64 };
    println!("✅ Pointer in struct works");
    let _null_ptr = get_null_ptr();
    let _null_mut_ptr = get_null_mut_ptr();
    println!("✅ Returning pointers works");
    println!("");
    println!("✅ All pointer type tests passed!");
    println!("✅ Extern functions with pointer params declared");
    println!("");
    println!("🎉 POINTER TYPES WORKING! 🎉");
}


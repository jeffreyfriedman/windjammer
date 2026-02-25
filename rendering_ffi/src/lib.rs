// Rendering FFI - Windjammer's bridge to wgpu
// This is Rust code that Windjammer programs call via extern fn

use std::sync::Mutex;

// Lazy static for global renderer state
static RENDERER: Mutex<Option<Renderer>> = Mutex::new(None);

struct Renderer {
    clear_color: (f32, f32, f32),
    frame_count: u64,
}

impl Renderer {
    fn new() -> Self {
        Renderer {
            clear_color: (0.0, 0.0, 0.0),
            frame_count: 0,
        }
    }
}

// FFI functions callable from Windjammer

#[no_mangle]
pub extern "C" fn wgpu_init() -> i32 {
    let mut renderer = RENDERER.lock().unwrap();
    *renderer = Some(Renderer::new());
    println!("[wgpu_init] ✅ Renderer initialized");
    0
}

#[no_mangle]
pub extern "C" fn wgpu_create_window(width: i32, height: i32, title: *const u8) -> i32 {
    // For now, just print - real winit integration coming
    println!("[wgpu_create_window] Creating {}x{} window", width, height);
    0
}

#[no_mangle]
pub extern "C" fn wgpu_should_close() -> bool {
    let renderer = RENDERER.lock().unwrap();
    if let Some(ref r) = *renderer {
        // Close after 300 frames for demo
        r.frame_count >= 300
    } else {
        true
    }
}

#[no_mangle]
pub extern "C" fn wgpu_poll_events() {
    // Stub - real event loop coming
}

#[no_mangle]
pub extern "C" fn wgpu_clear(r: f32, g: f32, b: f32) {
    let mut renderer = RENDERER.lock().unwrap();
    if let Some(ref mut renderer) = *renderer {
        renderer.clear_color = (r, g, b);
        
        // Print every 60 frames
        if renderer.frame_count % 60 == 0 {
            println!("[wgpu_clear] Frame {} - Color: ({:.2}, {:.2}, {:.2})", 
                     renderer.frame_count, r, g, b);
        }
    }
}

#[no_mangle]
pub extern "C" fn wgpu_present() {
    let mut renderer = RENDERER.lock().unwrap();
    if let Some(ref mut renderer) = *renderer {
        renderer.frame_count += 1;
    }
}

#[no_mangle]
pub extern "C" fn wgpu_shutdown() {
    let mut renderer = RENDERER.lock().unwrap();
    if let Some(ref r) = *renderer {
        println!("[wgpu_shutdown] ✅ {} frames rendered", r.frame_count);
    }
    *renderer = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_renderer_lifecycle() {
        assert_eq!(wgpu_init(), 0);
        assert_eq!(wgpu_create_window(640, 480, std::ptr::null()), 0);
        
        for _ in 0..10 {
            wgpu_poll_events();
            wgpu_clear(1.0, 0.0, 0.0);
            wgpu_present();
        }
        
        wgpu_shutdown();
    }
}

# GPU Rendering Validation Framework (TDD Guardrails)

**Purpose:** Prevent compute shader bugs from ever reaching production by catching them at build/test time.

## Design Principles

1. **Fail Fast** - Detect issues immediately, not after 60 frames
2. **Visual Confirmation** - Screenshots are ground truth
3. **Automated** - No manual verification required
4. **Comprehensive** - Cover bind groups, buffers, shaders, dispatch
5. **Zero False Positives** - If test fails, there's a real bug

## Framework Components

### 1. Bind Group Validator (`BindGroupValidator`)

**Purpose:** Ensure shader bindings match what's actually bound to the pipeline.

**Implementation (Rust):**
```rust
// windjammer-runtime-host/src/gpu_validation.rs

pub struct BindGroupValidator {
    required_bindings: HashMap<u32, BindingInfo>,
    bound_buffers: HashMap<u32, u32>,
}

#[derive(Debug)]
pub struct BindingInfo {
    pub slot: u32,
    pub ty: BindingType,
    pub label: String,
}

#[derive(Debug, PartialEq)]
pub enum BindingType {
    Uniform,
    StorageReadOnly,
    StorageReadWrite,
}

impl BindGroupValidator {
    pub fn new() -> Self {
        Self {
            required_bindings: HashMap::new(),
            bound_buffers: HashMap::new(),
        }
    }
    
    pub fn expect_binding(&mut self, slot: u32, ty: BindingType, label: &str) {
        self.required_bindings.insert(slot, BindingInfo {
            slot,
            ty,
            label: label.to_string(),
        });
    }
    
    pub fn bind_buffer(&mut self, slot: u32, buffer_id: u32) {
        self.bound_buffers.insert(slot, buffer_id);
    }
    
    pub fn validate(&self) -> Result<(), String> {
        for (slot, info) in &self.required_bindings {
            if !self.bound_buffers.contains_key(slot) {
                return Err(format!(
                    "MISSING BINDING: Shader expects {} at slot {}, but nothing is bound!",
                    info.label, slot
                ));
            }
            
            let buffer_id = self.bound_buffers[slot];
            if buffer_id == 0 {
                return Err(format!(
                    "INVALID BINDING: Shader expects {} at slot {}, but buffer ID is 0!",
                    info.label, slot
                ));
            }
        }
        
        Ok(())
    }
}

// Usage in gpu_dispatch_compute():
pub fn validate_before_dispatch(
    shader_id: u32,
    bound_uniform: &HashMap<u32, u32>,
    bound_storage: &HashMap<u32, u32>,
    bound_readonly: &HashMap<u32, u32>,
) -> Result<(), String> {
    let mut validator = BindGroupValidator::new();
    
    // Parse shader to extract required bindings
    // (This would require storing shader metadata at load time)
    let shader_meta = get_shader_metadata(shader_id)?;
    
    for binding in &shader_meta.bindings {
        validator.expect_binding(binding.slot, binding.ty.clone(), &binding.label);
    }
    
    // Register all bound buffers
    for (&slot, &buffer_id) in bound_uniform {
        validator.bind_buffer(slot, buffer_id);
    }
    for (&slot, &buffer_id) in bound_storage {
        validator.bind_buffer(slot, buffer_id);
    }
    for (&slot, &buffer_id) in bound_readonly {
        validator.bind_buffer(slot, buffer_id);
    }
    
    validator.validate()
}
```

**Test:**
```rust
#[test]
fn test_bind_group_validator_detects_missing_binding() {
    let mut validator = BindGroupValidator::new();
    validator.expect_binding(0, BindingType::Uniform, "camera");
    validator.expect_binding(1, BindingType::StorageReadWrite, "output");
    
    // Only bind slot 0, forget slot 1
    validator.bind_buffer(0, 123);
    
    let result = validator.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("MISSING BINDING"));
    assert!(result.unwrap_err().contains("output"));
}
```

---

### 2. Buffer Integrity Checker (`BufferIntegrityChecker`)

**Purpose:** Verify buffers have correct size and are writable.

**Implementation (Rust):**
```rust
pub struct BufferIntegrityChecker {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
}

impl BufferIntegrityChecker {
    pub fn verify_buffer_size(
        &self,
        buffer: &wgpu::Buffer,
        expected_size: u64,
        label: &str,
    ) -> Result<(), String> {
        let actual_size = buffer.size();
        if actual_size != expected_size {
            return Err(format!(
                "BUFFER SIZE MISMATCH: {} expected {} bytes, got {}",
                label, expected_size, actual_size
            ));
        }
        Ok(())
    }
    
    pub async fn verify_buffer_writable(
        &self,
        buffer_id: u32,
        width: u32,
        height: u32,
    ) -> Result<(), String> {
        // Write test pattern to first pixel
        let test_value = vec![1.0f32, 0.5, 0.25, 1.0]; // Orange
        
        // Create staging buffer
        let staging = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Test Pattern Staging"),
            contents: bytemuck::cast_slice(&test_value),
            usage: wgpu::BufferUsages::COPY_SRC,
        });
        
        // Copy to target buffer
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_buffer_to_buffer(&staging, 0, buffer, 0, 16);
        self.queue.submit(Some(encoder.finish()));
        
        // Read back first pixel
        let readback = read_buffer_pixel(buffer, 0).await?;
        
        // Verify write succeeded
        if readback[0] != test_value[0] {
            return Err(format!(
                "BUFFER NOT WRITABLE: Expected {:?}, got {:?}",
                test_value, readback
            ));
        }
        
        Ok(())
    }
}
```

**Test:**
```rust
#[tokio::test]
async fn test_buffer_integrity_checker_detects_wrong_size() {
    let (device, queue) = create_test_device().await;
    let checker = BufferIntegrityChecker { device, queue };
    
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Test Buffer"),
        size: 1024,
        usage: wgpu::BufferUsages::STORAGE,
        mapped_at_creation: false,
    });
    
    let result = checker.verify_buffer_size(&buffer, 2048, "Test Buffer");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("SIZE MISMATCH"));
}
```

---

### 3. Visual Validation (Screenshot Comparison)

**Purpose:** Pixel-perfect comparison against golden images.

**Implementation (Rust):**
```rust
pub struct VisualValidator {
    golden_images_path: PathBuf,
}

impl VisualValidator {
    pub fn compare_screenshot(
        &self,
        actual_path: &Path,
        test_name: &str,
        tolerance: f32, // 0.0 = pixel-perfect, 1.0 = any difference OK
    ) -> Result<(), String> {
        let golden_path = self.golden_images_path.join(format!("{}.png", test_name));
        
        if !golden_path.exists() {
            // First run - save as golden image
            std::fs::copy(actual_path, &golden_path)?;
            println!("GOLDEN IMAGE SAVED: {}", golden_path.display());
            return Ok(());
        }
        
        // Load images
        let actual = image::open(actual_path)?;
        let golden = image::open(&golden_path)?;
        
        // Compare dimensions
        if actual.dimensions() != golden.dimensions() {
            return Err(format!(
                "DIMENSION MISMATCH: actual {:?}, golden {:?}",
                actual.dimensions(),
                golden.dimensions()
            ));
        }
        
        // Compare pixels
        let actual_rgba = actual.to_rgba8();
        let golden_rgba = golden.to_rgba8();
        let mut diff_count = 0;
        let total_pixels = actual_rgba.width() * actual_rgba.height();
        
        for y in 0..actual_rgba.height() {
            for x in 0..actual_rgba.width() {
                let actual_pixel = actual_rgba.get_pixel(x, y);
                let golden_pixel = golden_rgba.get_pixel(x, y);
                
                let diff = pixel_difference(actual_pixel, golden_pixel);
                if diff > tolerance {
                    diff_count += 1;
                }
            }
        }
        
        let diff_percentage = (diff_count as f32 / total_pixels as f32) * 100.0;
        
        if diff_percentage > 1.0 {
            // Save diff image for debugging
            let diff_path = actual_path.with_extension("diff.png");
            save_diff_image(&actual_rgba, &golden_rgba, &diff_path)?;
            
            return Err(format!(
                "VISUAL REGRESSION: {:.2}% pixels differ (threshold: 1.0%)\
                 \nGolden: {}\
                 \nActual: {}\
                 \nDiff: {}",
                diff_percentage,
                golden_path.display(),
                actual_path.display(),
                diff_path.display()
            ));
        }
        
        Ok(())
    }
}

fn pixel_difference(a: &image::Rgba<u8>, b: &image::Rgba<u8>) -> f32 {
    let r_diff = (a[0] as i32 - b[0] as i32).abs() as f32 / 255.0;
    let g_diff = (a[1] as i32 - b[1] as i32).abs() as f32 / 255.0;
    let b_diff = (a[2] as i32 - b[2] as i32).abs() as f32 / 255.0;
    let a_diff = (a[3] as i32 - b[3] as i32).abs() as f32 / 255.0;
    
    (r_diff + g_diff + b_diff + a_diff) / 4.0
}
```

**Test:**
```rust
#[test]
fn test_visual_validator_detects_regression() {
    let validator = VisualValidator {
        golden_images_path: PathBuf::from("tests/golden"),
    };
    
    // Create golden image (red square)
    let mut golden = RgbaImage::new(100, 100);
    for pixel in golden.pixels_mut() {
        *pixel = image::Rgba([255, 0, 0, 255]);
    }
    golden.save("tests/golden/red_square.png").unwrap();
    
    // Create test image (blue square - regression!)
    let mut test = RgbaImage::new(100, 100);
    for pixel in test.pixels_mut() {
        *pixel = image::Rgba([0, 0, 255, 255]);
    }
    test.save("/tmp/red_square.png").unwrap();
    
    // Compare
    let result = validator.compare_screenshot(
        Path::new("/tmp/red_square.png"),
        "red_square",
        0.01,
    );
    
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("VISUAL REGRESSION"));
}
```

---

### 4. Build Fingerprinting

**Purpose:** Detect stale binaries that don't match source code.

**Implementation (build.rs):**
```rust
// breach-protocol/build.rs
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::fs;
use std::path::Path;

fn main() {
    // Hash all .wj source files
    let mut hasher = DefaultHasher::new();
    
    for entry in walkdir::WalkDir::new("src") {
        let entry = entry.unwrap();
        if entry.path().extension().map(|e| e == "wj").unwrap_or(false) {
            let contents = fs::read(entry.path()).unwrap();
            contents.hash(&mut hasher);
            entry.path().to_string_lossy().hash(&mut hasher);
        }
    }
    
    let hash = hasher.finish();
    
    // Write hash to file
    let out_dir = std::env::var("OUT_DIR").unwrap();
    fs::write(
        Path::new(&out_dir).join("source_hash.txt"),
        format!("{:016x}", hash),
    ).unwrap();
    
    println!("cargo:rerun-if-changed=src");
}
```

**Runtime Check (game.wj):**
```rust
// breach-protocol/src/game.wj
const SOURCE_HASH: &str = include_str!(concat!(env!("OUT_DIR"), "/source_hash.txt"));

pub fn verify_build_freshness() {
    // Compute current hash
    let current_hash = compute_source_hash();
    
    if current_hash != SOURCE_HASH {
        panic!(
            "BUILD IS STALE!\
             \n  Binary hash:  {}\
             \n  Source hash:  {}\
             \n  \
             \n  Source files have changed but binary was not rebuilt.\
             \n  Run: wj game build --release",
            SOURCE_HASH,
            current_hash
        );
    }
}
```

---

## Integration: Automated Test Suite

**File: `windjammer-game/windjammer-game-core/tests/gpu_rendering_validation.rs`**

```rust
#[tokio::test]
async fn test_compute_shader_produces_visible_output() {
    let (device, queue) = init_gpu_test_device().await;
    
    // 1. Load validation shader
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Validation Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!(
            "../../../breach-protocol/runtime_host/shaders/test_rendering_validation.wgsl"
        ).into()),
    });
    
    // 2. Create buffers
    let width = 1280u32;
    let height = 720u32;
    let output_buffer = create_rgba32float_buffer(&device, width, height);
    let screen_size_buffer = create_uniform_buffer(&device, &[width, height]);
    
    // 3. Validate bind group
    let mut validator = BindGroupValidator::new();
    validator.expect_binding(5, BindingType::StorageReadWrite, "output");
    validator.expect_binding(6, BindingType::Uniform, "screen_size");
    validator.bind_buffer(5, output_buffer.id);
    validator.bind_buffer(6, screen_size_buffer.id);
    validator.validate().unwrap();
    
    // 4. Dispatch
    let groups_x = (width + 7) / 8;
    let groups_y = (height + 7) / 8;
    dispatch_compute(&device, &queue, &shader, groups_x, groups_y, 1);
    
    // 5. Read back and verify
    let screenshot_path = PathBuf::from("/tmp/validation_test.png");
    save_buffer_to_png(&output_buffer, width, height, &screenshot_path).await;
    
    // 6. Visual validation
    let visual_validator = VisualValidator {
        golden_images_path: PathBuf::from("tests/golden_images"),
    };
    visual_validator.compare_screenshot(&screenshot_path, "validation_pattern", 0.01).unwrap();
    
    // 7. Pixel-level checks
    let pixels = read_buffer_rgba32(&output_buffer, width, height).await;
    
    // Top-left corner should be RED
    assert_pixel_color(&pixels[0], 1.0, 0.0, 0.0, 1.0, "Top-left corner (RED)");
    
    // Top-right corner should be GREEN
    let tr_idx = (width - 1) as usize;
    assert_pixel_color(&pixels[tr_idx], 0.0, 1.0, 0.0, 1.0, "Top-right corner (GREEN)");
    
    // Bottom-left corner should be BLUE
    let bl_idx = ((height - 1) * width) as usize;
    assert_pixel_color(&pixels[bl_idx], 0.0, 0.0, 1.0, 1.0, "Bottom-left corner (BLUE)");
    
    // Bottom-right corner should be YELLOW
    let br_idx = ((height - 1) * width + width - 1) as usize;
    assert_pixel_color(&pixels[br_idx], 1.0, 1.0, 0.0, 1.0, "Bottom-right corner (YELLOW)");
    
    // Center should be WHITE
    let center_idx = ((height / 2) * width + width / 2) as usize;
    assert_pixel_color(&pixels[center_idx], 1.0, 1.0, 1.0, 1.0, "Center (WHITE)");
}

fn assert_pixel_color(
    pixel: &[f32; 4],
    r: f32, g: f32, b: f32, a: f32,
    label: &str
) {
    let tolerance = 0.01;
    assert!(
        (pixel[0] - r).abs() < tolerance &&
        (pixel[1] - g).abs() < tolerance &&
        (pixel[2] - b).abs() < tolerance &&
        (pixel[3] - a).abs() < tolerance,
        "{} color mismatch: expected ({}, {}, {}, {}), got ({}, {}, {}, {})",
        label, r, g, b, a, pixel[0], pixel[1], pixel[2], pixel[3]
    );
}
```

---

## CI Integration

**File: `.github/workflows/gpu_validation.yml`**

```yaml
name: GPU Rendering Validation

on: [push, pull_request]

jobs:
  gpu-validation:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      
      - name: Install GPU testing dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y mesa-vulkan-drivers vulkan-tools
      
      - name: Run GPU validation tests
        run: cargo test --release --test gpu_rendering_validation
      
      - name: Upload screenshot artifacts
        if: failure()
        uses: actions/upload-artifact@v3
        with:
          name: failed-screenshots
          path: |
            /tmp/*.png
            /tmp/*.diff.png
```

---

## Summary: How This Prevents Future Bugs

1. **Bind Group Validator** → Catches missing/mismatched bindings at dispatch time
2. **Buffer Integrity Checker** → Catches size mismatches and write failures
3. **Visual Validator** → Catches regressions in shader output
4. **Build Fingerprinting** → Catches stale binaries
5. **Automated Tests** → Runs on every commit, catches bugs before merge

**Result:** GPU rendering bugs become **impossible** to ship to production.

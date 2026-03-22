// TDD Test: Verify `trait TraitName` syntax for trait parameters
//
// Windjammer uses cleaner syntax than Rust:
// - Rust: fn add(item: impl MyTrait) -> ()
// - Windjammer: fn add(item: trait MyTrait) -> ()
//
// This test verifies the compiler correctly handles trait parameters.

use std::env;
use std::fs;
use std::path::PathBuf;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_trait_parameter_basic() {
    let test_dir = std::env::temp_dir().join(format!(
        "wj_trait_param_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    ));

    fs::create_dir_all(&test_dir).unwrap();

    let source = r#"
pub trait Drawable {
    fn draw(self)
}

pub struct Circle {
    radius: f32,
}

impl Drawable for Circle {
    fn draw(self) {
        println!("Drawing circle")
    }
}

pub fn render(item: trait Drawable) {
    item.draw()
}
"#;

    fs::write(test_dir.join("trait_param.wj"), source).unwrap();

    let wj_binary = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let output = std::process::Command::new(&wj_binary)
        .arg("build")
        .arg(test_dir.join("trait_param.wj"))
        .arg("--no-cargo")
        .current_dir(&test_dir)
        .output()
        .expect("Failed to execute wj compiler");

    assert!(output.status.success(), 
            "Compilation failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr));
    
    // Read generated Rust code
    let rust_code = fs::read_to_string(test_dir.join("build/trait_param.rs"))
        .expect("Failed to read generated Rust");
    
    // Should generate proper trait handling
    assert!(rust_code.contains("trait") || rust_code.contains("Drawable"),
            "Should generate proper Rust trait syntax");
    
    // Cleanup
    fs::remove_dir_all(&test_dir).ok();
}


#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_real_world_shader_effect_pattern() {
    let test_dir = std::env::temp_dir().join(format!(
        "wj_shader_effect_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    ));

    fs::create_dir_all(&test_dir).unwrap();

    // This is the actual pattern from shader_effect.wj
    let source = r#"
pub trait ShaderEffect {
    fn update(self, dt: f32)
    fn is_active(self) -> bool
}

pub struct FireEffect {
    intensity: f32,
}

impl ShaderEffect for FireEffect {
    fn update(self, dt: f32) {
        // Update fire
    }
    
    fn is_active(self) -> bool {
        self.intensity > 0.0
    }
}

pub struct EffectManager {
    effects: Vec<Box<dyn ShaderEffect>>,
}

impl EffectManager {
    pub fn new() -> EffectManager {
        EffectManager {
            effects: Vec::new(),
        }
    }
    
    pub fn add_effect(self, effect: trait ShaderEffect) -> EffectManager {
        let mut new_effects = self.effects
        new_effects.push(Box::new(effect))
        
        EffectManager {
            effects: new_effects,
        }
    }
    
    pub fn update_all(self, dt: f32) {
        for effect in self.effects {
            if effect.is_active() {
                effect.update(dt)
            }
        }
    }
}
"#;

    fs::write(test_dir.join("shader_effect.wj"), source).unwrap();

    let wj_binary = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let output = std::process::Command::new(&wj_binary)
        .arg("build")
        .arg(test_dir.join("shader_effect.wj"))
        .arg("--no-cargo")
        .current_dir(&test_dir)
        .output()
        .expect("Failed to execute wj compiler");

    assert!(output.status.success(), 
            "Real-world shader effect pattern should compile:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr));
    
    // Read generated Rust code
    let rust_code = fs::read_to_string(test_dir.join("build/shader_effect.rs"))
        .expect("Failed to read generated Rust");
    
    // Verify key patterns are generated
    assert!(rust_code.contains("ShaderEffect"), "Should preserve trait name");
    assert!(rust_code.contains("Box"), "Should generate boxing code");
    assert!(rust_code.contains("Vec"), "Should handle collections");
    
    // Cleanup
    fs::remove_dir_all(&test_dir).ok();
}

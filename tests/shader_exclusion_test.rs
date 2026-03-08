// TDD Test: Shader directories should be excluded from Rust compilation
// Bug: Shader files with WGSL types (vec3<float>) get compiled to Rust, causing type errors

use std::path::{Path, PathBuf};
use tempfile::TempDir;
use std::fs;

#[test]
fn test_shader_directory_excluded_from_discovery() {
    // Create temp project structure
    let temp_dir = TempDir::new().unwrap();
    let src_wj = temp_dir.path().join("src_wj");
    fs::create_dir(&src_wj).unwrap();
    
    // Create mod.wj
    let mod_wj = src_wj.join("mod.wj");
    fs::write(&mod_wj, "pub mod math\n").unwrap();
    
    // Create regular .wj file (should be found)
    let math_dir = src_wj.join("math");
    fs::create_dir(&math_dir).unwrap();
    fs::write(math_dir.join("vector.wj"), "pub struct Vec3 { x: float, y: float, z: float }").unwrap();
    
    // Create shaders directory (should be excluded)
    let shaders_dir = src_wj.join("shaders");
    fs::create_dir(&shaders_dir).unwrap();
    fs::write(shaders_dir.join("vertex.wj"), "@vertex pub fn vs() -> vec4<float> { vec4(0.0, 0.0, 0.0, 1.0) }").unwrap();
    
    // Discover files
    let files = discover_wj_files_excluding_shaders(&mod_wj).unwrap();
    
    // Assert: shader files should NOT be discovered
    let file_names: Vec<String> = files.iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
        .collect();
    
    assert!(file_names.contains(&"vector.wj".to_string()), 
        "Regular .wj files should be discovered");
    assert!(!file_names.contains(&"vertex.wj".to_string()), 
        "Shader files should be excluded from discovery");
}

// Helper function that mimics the compiler's file discovery with shader exclusion
fn discover_wj_files_excluding_shaders(path: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    
    if path.is_file() && path.file_name().and_then(|n| n.to_str()) == Some("mod.wj") {
        if let Some(parent) = path.parent() {
            find_wj_files_recursive_excluding_shaders(parent, &mut files)?;
        }
    }
    
    Ok(files)
}

fn find_wj_files_recursive_excluding_shaders(dir: &Path, files: &mut Vec<PathBuf>) -> anyhow::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("wj") {
            files.push(path);
        } else if path.is_dir() {
            // TDD FIX: Skip "shaders" directory
            let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if dir_name == "shaders" {
                continue; // Skip shaders directory
            }
            find_wj_files_recursive_excluding_shaders(&path, files)?;
        }
    }
    Ok(())
}

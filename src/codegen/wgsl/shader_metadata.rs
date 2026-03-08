// WGSL Shader Metadata Parser
//
// Parses WGSL shader files at compile-time to extract binding metadata.
// This enables the ShaderGraph system to validate bindings and infer slot numbers.
//
// Extracts:
// - @group(N) @binding(M) declarations
// - var<uniform>, var<storage, read>, var<storage, read_write>
// - Struct type names
// - Entry points (@compute, @vertex, @fragment)
//
// Used by windjammer-game's ShaderGraph builder for compile-time validation.


#[derive(Debug, Clone, PartialEq)]
pub struct ShaderMetadata {
    pub bindings: Vec<Binding>,
    pub entry_points: Vec<EntryPoint>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Binding {
    pub group: u32,
    pub binding: u32,
    pub name: String,
    pub binding_type: BindingType,
    pub wgsl_type: String,  // "CameraUniforms", "array<vec4<f32>>", etc.
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindingType {
    Uniform,
    StorageRead,
    StorageReadWrite,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntryPoint {
    pub name: String,
    pub stage: ShaderStage,
    pub workgroup_size: Option<(u32, u32, u32)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShaderStage {
    Compute,
    Vertex,
    Fragment,
}

/// Parse WGSL source and extract binding metadata
pub fn extract_shader_metadata(wgsl_source: &str) -> ShaderMetadata {
    let mut bindings = Vec::new();
    let mut entry_points = Vec::new();
    
    let lines: Vec<&str> = wgsl_source.lines().collect();
    let mut i = 0;
    
    while i < lines.len() {
        let line = lines[i].trim();
        
        // Skip empty lines and comments
        if line.is_empty() || line.starts_with("//") {
            i += 1;
            continue;
        }
        
        // Parse @group and @binding attributes (look ahead for var declaration)
        if line.contains("@group") && line.contains("@binding") {
            // Single line: @group(0) @binding(1) var<uniform> camera: CameraUniforms;
            if let Some(binding) = parse_single_line_binding(line) {
                bindings.push(binding);
            }
        } else if line.starts_with("@group") {
            // Multi-line: @group on one line, @binding and/or var on next lines
            if let Some(binding) = parse_binding(&lines, i) {
                bindings.push(binding);
            }
        }
        
        // Parse entry points
        if line.contains("@compute") || line.contains("@vertex") || line.contains("@fragment") {
            if let Some(entry) = parse_entry_point(&lines, &mut i) {
                entry_points.push(entry);
            }
        }
        
        i += 1;
    }
    
    ShaderMetadata {
        bindings,
        entry_points,
    }
}

/// Parse binding from a single line
/// Example: @group(0) @binding(1) var<uniform> camera: CameraUniforms;
fn parse_single_line_binding(line: &str) -> Option<Binding> {
    let group = extract_number(line, "@group(", ")")?;
    let binding = extract_number(line, "@binding(", ")")?;
    parse_var_declaration(line, group, binding)
}

/// Parse multi-line binding declaration  
/// Example:
///   @group(0)
///   @binding(1)
///   var<uniform> camera: CameraUniforms;
fn parse_binding(lines: &[&str], start_index: usize) -> Option<Binding> {
    let mut group = None;
    let mut binding = None;
    let mut var_line_idx = start_index;
    
    // Scan forward from start_index looking for group, binding, and var
    for offset in 0..5 {
        if start_index + offset >= lines.len() {
            break;
        }
        let line = lines[start_index + offset].trim();
        
        if line.contains("@group") && group.is_none() {
            group = extract_number(line, "@group(", ")");
        }
        
        if line.contains("@binding") && binding.is_none() {
            binding = extract_number(line, "@binding(", ")");
        }
        
        if line.starts_with("var<") {
            var_line_idx = start_index + offset;
            break;
        }
    }
    
    let var_line = lines[var_line_idx].trim();
    if var_line.starts_with("var<") && group.is_some() && binding.is_some() {
        return parse_var_declaration(var_line, group?, binding?);
    }
    
    None
}

/// Parse var<storage_class> name: Type;
fn parse_var_declaration(line: &str, group: u32, binding: u32) -> Option<Binding> {
    // Extract storage class: var<uniform>, var<storage, read>, var<storage, read_write>
    let binding_type = if line.contains("uniform") {
        BindingType::Uniform
    } else if line.contains("read_write") {
        BindingType::StorageReadWrite
    } else if line.contains("storage") && line.contains("read") {
        BindingType::StorageRead
    } else {
        return None;
    };
    
    // Extract name and type
    // Format: var<...> name: Type;
    // Find the closing > of var<...> (not nested generics in the type)
    let var_start = line.find("var<")?;
    let after_var_keyword = &line[var_start + 4..]; // Skip "var<"
    
    // Find matching > for var<...>
    let mut depth = 0;
    let mut var_end_relative = None;
    for (i, ch) in after_var_keyword.chars().enumerate() {
        match ch {
            '<' => depth += 1,
            '>' => {
                if depth == 0 {
                    var_end_relative = Some(i);
                    break;
                }
                depth -= 1;
            },
            _ => {}
        }
    }
    
    let var_end_idx = var_start + 4 + var_end_relative? + 1;
    let after_var = &line[var_end_idx..];
    
    let parts: Vec<&str> = after_var.split(':').collect();
    if parts.len() < 2 {
        return None;
    }
    
    let name = parts[0].trim().to_string();
    let wgsl_type = parts[1].trim().trim_end_matches(';').trim().to_string();
    
    Some(Binding {
        group,
        binding,
        name,
        binding_type,
        wgsl_type,
    })
}

/// Parse entry point with @compute/@vertex/@fragment
fn parse_entry_point(lines: &[&str], index: &mut usize) -> Option<EntryPoint> {
    let line = lines[*index].trim();
    
    let stage = if line.contains("@compute") {
        ShaderStage::Compute
    } else if line.contains("@vertex") {
        ShaderStage::Vertex
    } else if line.contains("@fragment") {
        ShaderStage::Fragment
    } else {
        return None;
    };
    
    // Extract workgroup_size for compute shaders
    let workgroup_size = if stage == ShaderStage::Compute {
        extract_workgroup_size(line)
    } else {
        None
    };
    
    // Find function name (next line or same line)
    let fn_line = if line.contains("fn ") {
        line
    } else if *index + 1 < lines.len() {
        lines[*index + 1].trim()
    } else {
        return None;
    };
    
    let name = extract_function_name(fn_line)?;
    
    Some(EntryPoint {
        name,
        stage,
        workgroup_size,
    })
}

/// Extract number from pattern like "@group(5)"
fn extract_number(line: &str, prefix: &str, suffix: &str) -> Option<u32> {
    let start = line.find(prefix)? + prefix.len();
    let end = line[start..].find(suffix)? + start;
    let num_str = &line[start..end];
    num_str.trim().parse().ok()
}

/// Extract workgroup_size from @workgroup_size(x, y, z)
fn extract_workgroup_size(line: &str) -> Option<(u32, u32, u32)> {
    let start = line.find("@workgroup_size(")? + 16;
    let end = line[start..].find(')')? + start;
    let nums: Vec<&str> = line[start..end].split(',').collect();
    
    if nums.len() == 3 {
        let x = nums[0].trim().parse().ok()?;
        let y = nums[1].trim().parse().ok()?;
        let z = nums[2].trim().parse().ok()?;
        Some((x, y, z))
    } else {
        None
    }
}

/// Extract function name from "fn main(...)" or "fn compute_shader(...)"
fn extract_function_name(line: &str) -> Option<String> {
    let start = line.find("fn ")? + 3;
    let rest = &line[start..];
    let end = rest.find('(')?;
    Some(rest[..end].trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_single_line_uniform() {
        let source = "@group(0) @binding(0) var<uniform> camera: CameraUniforms;";
        
        let metadata = extract_shader_metadata(source);
        assert_eq!(metadata.bindings.len(), 1);
        
        let binding = &metadata.bindings[0];
        assert_eq!(binding.group, 0);
        assert_eq!(binding.binding, 0);
        assert_eq!(binding.name, "camera");
        assert_eq!(binding.binding_type, BindingType::Uniform);
        assert_eq!(binding.wgsl_type, "CameraUniforms");
    }
    
    #[test]
    fn test_parse_single_line_storage_read() {
        let source = "@group(0) @binding(2) var<storage, read> svo_nodes: array<u32>;";
        
        let metadata = extract_shader_metadata(source);
        assert_eq!(metadata.bindings.len(), 1, "Expected to parse 1 binding");
        
        let binding = &metadata.bindings[0];
        assert_eq!(binding.binding, 2);
        assert_eq!(binding.binding_type, BindingType::StorageRead);
        assert_eq!(binding.wgsl_type, "array<u32>");
    }
    
    #[test]
    fn test_parse_compute_entry_point() {
        let source = r#"
@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
}
        "#;
        
        let metadata = extract_shader_metadata(source);
        assert_eq!(metadata.entry_points.len(), 1);
        
        let entry = &metadata.entry_points[0];
        assert_eq!(entry.name, "main");
        assert_eq!(entry.stage, ShaderStage::Compute);
        assert_eq!(entry.workgroup_size, Some((8, 8, 1)));
    }
}

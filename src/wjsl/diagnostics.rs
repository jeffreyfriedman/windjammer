//! WJSL shader-specific diagnostics and error hints (WJ-LANG-01).

/// Format a type-check error with WJSL context and optional fix hints.
pub fn format_shader_error(message: &str, source: &str) -> String {
    let mut out = format!("WJSL error: {message}");
    if let Some(hint) = hint_for_message(message, source) {
        out.push_str("\n  hint: ");
        out.push_str(&hint);
    }
    out
}

/// Wrap an anyhow error chain with shader-specific hints.
pub fn enrich_shader_error(err: anyhow::Error, source: &str) -> anyhow::Error {
    let msg = err.to_string();
    anyhow::anyhow!("{}", format_shader_error(&msg, source))
}

fn hint_for_message(message: &str, source: &str) -> Option<String> {
    if let Some(name) = message.strip_prefix("Unknown identifier '").and_then(|rest| {
        rest.strip_suffix('\'')
    }) {
        return suggest_unknown_identifier(name, source);
    }

    if message.contains("Duplicate @binding") {
        return Some(
            "Each @group(N) @binding(M) slot must be unique. Renumber bindings or merge resources."
                .to_string(),
        );
    }

    if message.contains("Cannot index type") {
        return Some(
            "Only arrays, vectors, and matrices support `[index]`. Check binding type or cast first."
                .to_string(),
        );
    }

    if message.contains("vector sizes must match") {
        return Some(
            "WJSL requires matching vector sizes for add/subtract. Use explicit casts like vec3(f32, f32, f32)."
                .to_string(),
        );
    }

    if message.contains("Return type mismatch") {
        return Some(
            "Entry point return type must match the declared `@location` / struct output type."
                .to_string(),
        );
    }

    if message.contains("Circular #include") || message.contains("Circular use") {
        return Some(
            "Split shared types into a leaf header (e.g. wjsl_std/types.wjsl) included by both shaders."
                .to_string(),
        );
    }

    None
}

/// Suggest closest symbol names for unknown identifiers (Levenshtein distance).
pub fn suggest_unknown_identifier(name: &str, source: &str) -> Option<String> {
    let candidates = collect_identifier_candidates(source);
    if candidates.is_empty() {
        return Some(format!(
            "'{name}' is not declared. Declare it as a binding, struct field, let, or fn parameter."
        ));
    }

    let mut scored: Vec<(u32, &str)> = candidates
        .iter()
        .map(|c| (levenshtein(name, c), *c))
        .filter(|(d, _)| *d <= 3)
        .collect();
    scored.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(b.1)));

    if let Some((_, best)) = scored.first() {
        Some(format!("did you mean '{best}'?"))
    } else {
        Some(format!(
            "'{name}' is not in scope. Available bindings and locals include: {}",
            candidates
                .iter()
                .take(8)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }
}

fn collect_identifier_candidates(source: &str) -> Vec<&str> {
    let mut names = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if (trimmed.starts_with('@') || trimmed.starts_with("uniform "))
            && trimmed.contains("uniform ")
        {
            if let Some(name) = uniform_binding_name(trimmed) {
                names.push(name);
            }
        }
        if (trimmed.starts_with('@') || trimmed.starts_with("storage "))
            && trimmed.contains(" storage ")
        {
            if let Some(name) = storage_binding_name(trimmed) {
                names.push(name);
            }
        }
        if trimmed.starts_with("struct ") {
            if let Some(name) = trimmed.split_whitespace().nth(1) {
                names.push(name.trim_end_matches('{').trim_end_matches(' '));
            }
        }
        if trimmed.starts_with("fn ") {
            if let Some(name) = trimmed.split_whitespace().nth(1) {
                names.push(name.trim_end_matches('('));
            }
        }
        if trimmed.starts_with("let ") || trimmed.starts_with("var ") {
            if let Some(name) = trimmed.split_whitespace().nth(1) {
                names.push(name.trim_end_matches(':'));
            }
        }
    }
    names.sort_unstable();
    names.dedup();
    names
}

fn uniform_binding_name(line: &str) -> Option<&str> {
    let after_uniform = line.split("uniform ").nth(1)?;
    let name_part = after_uniform.split(':').next()?.trim();
    name_part.split_whitespace().next()
}

fn storage_binding_name(line: &str) -> Option<&str> {
    let after_storage = line.split(" storage ").nth(1)?;
    let rest = after_storage
        .trim_start_matches("read ")
        .trim_start_matches("write ")
        .trim_start_matches("read_write ");
    rest.split(':').next()?.trim().split_whitespace().next()
}

fn levenshtein(a: &str, b: &str) -> u32 {
    if a == b {
        return 0;
    }
    if a.is_empty() {
        return b.chars().count() as u32;
    }
    if b.is_empty() {
        return a.chars().count() as u32;
    }

    let mut prev: Vec<u32> = (0..=b.chars().count()).map(|i| i as u32).collect();
    let mut curr = vec![0u32; b.chars().count() + 1];

    for (i, ca) in a.chars().enumerate() {
        curr[0] = (i + 1) as u32;
        for (j, cb) in b.chars().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            curr[j + 1] = (prev[j + 1] + 1)
                .min(curr[j] + 1)
                .min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[b.chars().count()]
}

/// Auto-import common WJSL stdlib paths when referenced identifiers are missing from source.
///
/// Inserts `use "wjsl_std/..."` lines before the shader body when common GPU types are referenced
/// but no include/use directive is present. Called by build tools, not the transpile pipeline.
#[allow(dead_code)]
pub fn inject_common_std_imports(source: &str) -> String {
    let mut imports = Vec::new();
    let has_use = |path: &str| source.contains(path) || source.contains(&path.replace('/', "\\"));

    let needs_gpu_types = ["CameraUniforms", "GBufferPixel", "GpuVertex"]
        .iter()
        .any(|sym| source.contains(sym));
    if needs_gpu_types && !has_use("wjsl_std/gpu_types_generated.wjsl") {
        imports.push("use \"wjsl_std/gpu_types_generated.wjsl\"");
    }

    if source.contains("view_matrix") && !has_use("wjsl_std/camera.wjsl") {
        imports.push("use \"wjsl_std/camera.wjsl\"");
    }

    if imports.is_empty() {
        return source.to_string();
    }

    let mut out = imports.join("\n");
    out.push('\n');
    out.push_str(source);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggest_close_identifier() {
        let source = "uniform camera: CameraUniforms;\nfn main() { let x = camra; }";
        let hint = suggest_unknown_identifier("camra", source).unwrap();
        assert!(hint.contains("camera"));
    }

    #[test]
    fn inject_gpu_types_import_when_needed() {
        let source = "fn main() { let c: CameraUniforms; }";
        let injected = inject_common_std_imports(source);
        assert!(injected.contains("wjsl_std/gpu_types_generated.wjsl"));
    }

    #[test]
    fn format_shader_error_adds_binding_hint() {
        let msg = "Duplicate @binding(0) in @group(0): 'a' conflicts with 'b'";
        let formatted = format_shader_error(msg, "");
        assert!(formatted.contains("hint:"));
        assert!(formatted.contains("@group"));
    }
}

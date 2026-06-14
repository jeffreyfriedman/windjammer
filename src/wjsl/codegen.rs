//! WJSL → WGSL Codegen
//!
//! Converts WJSL AST to WGSL source code.

use crate::wjsl::ast::*;
use anyhow::Result;

/// WGSL code generator for WJSL shader modules
pub struct WjslCodegen {
    module: ShaderModule,
}

impl WjslCodegen {
    pub fn new(module: ShaderModule) -> Self {
        Self { module }
    }

    /// Generate WGSL source code from the shader module
    pub fn generate(&self) -> Result<String> {
        let mut out = String::new();

        // 1. Struct declarations
        for s in &self.module.structs {
            self.emit_struct(&mut out, s);
        }

        // 2. Global bindings
        for b in &self.module.bindings {
            self.emit_binding(&mut out, b);
        }

        // 2b. Constant declarations
        for cd in &self.module.const_decls {
            if let Some(ref ty) = cd.ty {
                out.push_str(&format!(
                    "const {}: {} = {};\n",
                    cd.name,
                    self.type_to_wgsl(ty),
                    cd.initializer
                ));
            } else {
                out.push_str(&format!("const {} = {};\n", cd.name, cd.initializer));
            }
        }
        if !self.module.const_decls.is_empty() {
            out.push('\n');
        }

        // 2c. Module-level variables (private and workgroup)
        for pv in &self.module.private_vars {
            let space = match pv.address_space {
                crate::wjsl::ast::AddressSpace::Private => "private",
                crate::wjsl::ast::AddressSpace::Workgroup => "workgroup",
            };
            out.push_str(&format!("var<{}> ", space));
            out.push_str(&pv.name);
            out.push_str(": ");
            out.push_str(&self.type_to_wgsl(&pv.ty));
            out.push_str(";\n");
        }
        if !self.module.private_vars.is_empty() {
            out.push('\n');
        }

        // 3. Helper functions
        for f in &self.module.functions {
            self.emit_function(&mut out, f);
        }

        // 4. Entry points
        for ep in &self.module.entry_points {
            self.emit_entry_point(&mut out, ep);
        }

        Ok(out)
    }

    fn emit_struct(&self, out: &mut String, s: &StructDecl) {
        out.push_str("struct ");
        out.push_str(&s.name);
        out.push_str(" {\n");
        for f in &s.fields {
            out.push_str("    ");
            if let Some(align) = f.align {
                out.push_str(&format!("@align({}) ", align));
            }
            if let Some(size) = f.size {
                out.push_str(&format!("@size({}) ", size));
            }
            if let Some(loc) = f.location {
                out.push_str(&format!("@location({}) ", loc));
            }
            if let Some(builtin) = &f.builtin {
                out.push_str(&format!("@builtin({}) ", builtin));
            }
            out.push_str(&format!("{}: ", f.name));
            out.push_str(&self.type_to_wgsl(&f.ty));
            out.push_str(",\n");
        }
        out.push_str("}\n\n");
    }

    fn emit_binding(&self, out: &mut String, b: &Binding) {
        out.push_str(&format!("@group({}) @binding({}) ", b.group, b.binding));
        match &b.kind {
            BindingKind::Uniform(ty) => {
                out.push_str("var<uniform> ");
                out.push_str(&b.name);
                out.push_str(": ");
                out.push_str(&self.type_to_wgsl(ty));
            }
            BindingKind::Storage { access_mode, ty } => {
                let access = match access_mode {
                    StorageAccess::Read => "storage, read",
                    StorageAccess::Write => "storage, write",
                    StorageAccess::ReadWrite => "storage, read_write",
                };
                out.push_str(&format!("var<{}> ", access));
                out.push_str(&b.name);
                out.push_str(": ");
                out.push_str(&self.type_to_wgsl(ty));
            }
            BindingKind::Texture { texture_type } => {
                out.push_str("var ");
                out.push_str(&b.name);
                out.push_str(": ");
                out.push_str(&self.texture_type_to_wgsl(texture_type));
            }
            BindingKind::Sampler => {
                out.push_str("var ");
                out.push_str(&b.name);
                out.push_str(": sampler");
            }
        }
        out.push_str(";\n");
    }

    fn texture_type_to_wgsl(&self, tt: &TextureType) -> String {
        match tt {
            TextureType::Texture2D(st) => format!("texture_2d<{}>", self.scalar_to_wgsl(*st)),
            TextureType::TextureCube(st) => format!("texture_cube<{}>", self.scalar_to_wgsl(*st)),
            TextureType::Texture3D(st) => format!("texture_3d<{}>", self.scalar_to_wgsl(*st)),
        }
    }

    fn scalar_to_wgsl(&self, st: ScalarType) -> &'static str {
        match st {
            ScalarType::F32 => "f32",
            ScalarType::F64 => "f64",
            ScalarType::U32 => "u32",
            ScalarType::I32 => "i32",
            ScalarType::Bool => "bool",
        }
    }

    fn type_to_wgsl(&self, ty: &Type) -> String {
        match ty {
            Type::Scalar(st) => self.scalar_to_wgsl(*st).to_string(),
            Type::Vec2(elem) => {
                let e = elem.unwrap_or(ScalarType::F32);
                format!("vec2<{}>", self.scalar_to_wgsl(e))
            }
            Type::Vec3(elem) => {
                let e = elem.unwrap_or(ScalarType::F32);
                format!("vec3<{}>", self.scalar_to_wgsl(e))
            }
            Type::Vec4(elem) => {
                let e = elem.unwrap_or(ScalarType::F32);
                format!("vec4<{}>", self.scalar_to_wgsl(e))
            }
            Type::Mat2x2(elem) => {
                let e = elem.unwrap_or(ScalarType::F32);
                format!("mat2x2<{}>", self.scalar_to_wgsl(e))
            }
            Type::Mat3x3(elem) => {
                let e = elem.unwrap_or(ScalarType::F32);
                format!("mat3x3<{}>", self.scalar_to_wgsl(e))
            }
            Type::Mat4x4(elem) => {
                let e = elem.unwrap_or(ScalarType::F32);
                format!("mat4x4<{}>", self.scalar_to_wgsl(e))
            }
            Type::Array(inner, size) => {
                if let Some(n) = size {
                    format!("array<{}, {}>", self.type_to_wgsl(inner), n)
                } else {
                    format!("array<{}>", self.type_to_wgsl(inner))
                }
            }
            Type::Atomic(st) => format!("atomic<{}>", self.scalar_to_wgsl(*st)),
            Type::Struct(name) => name.clone(),
            Type::Texture2D(st) => format!("texture_2d<{}>", self.scalar_to_wgsl(*st)),
            Type::TextureCube(st) => format!("texture_cube<{}>", self.scalar_to_wgsl(*st)),
            Type::Texture3D(st) => format!("texture_3d<{}>", self.scalar_to_wgsl(*st)),
            Type::Sampler => "sampler".to_string(),
            Type::SamplerComparison => "sampler_comparison".to_string(),
        }
    }

    fn emit_function(&self, out: &mut String, f: &Function) {
        out.push_str("fn ");
        out.push_str(&f.name);
        out.push('(');
        for (i, p) in f.params.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            self.emit_param(out, p);
        }
        out.push(')');
        if let Some(ref ret) = f.return_type {
            out.push_str(" -> ");
            out.push_str(&self.type_to_wgsl(ret));
        }
        out.push_str(" {\n");
        Self::emit_body(out, &f.body);
        out.push_str("}\n\n");
    }

    fn emit_body(out: &mut String, body: &str) {
        if body.is_empty() {
            return;
        }
        let lowered = lower_if_expressions(body);
        for line in lowered.lines() {
            out.push_str("    ");
            let transformed = line.replace("let mut ", "var ");
            out.push_str(&transformed);
            out.push('\n');
        }
    }

    fn emit_param(&self, out: &mut String, p: &Param) {
        if let Some(loc) = p.location {
            out.push_str(&format!("@location({}) ", loc));
        }
        if let Some(ref builtin) = p.builtin {
            out.push_str(&format!("@builtin({}) ", builtin));
        }
        out.push_str(&p.name);
        out.push_str(": ");
        out.push_str(&self.type_to_wgsl(&p.ty));
    }

    fn emit_entry_point(&self, out: &mut String, ep: &EntryPoint) {
        match ep.stage {
            ShaderStage::Vertex => out.push_str("@vertex\n"),
            ShaderStage::Fragment => out.push_str("@fragment\n"),
            ShaderStage::Compute => {
                out.push_str("@compute ");
                if let Some((x, y, z)) = ep.workgroup_size {
                    out.push_str(&format!("@workgroup_size({}, {}, {})\n", x, y, z));
                } else {
                    out.push('\n');
                }
            }
        }
        out.push_str("fn ");
        out.push_str(&ep.name);
        out.push('(');
        for (i, p) in ep.params.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            self.emit_param(out, p);
        }
        out.push(')');
        if let Some(ref ret) = ep.return_type {
            out.push_str(" -> ");
            if let Some(loc) = ret.location {
                out.push_str(&format!("@location({}) ", loc));
            }
            if let Some(ref builtin) = ret.builtin {
                out.push_str(&format!("@builtin({}) ", builtin));
            }
            out.push_str(&self.type_to_wgsl(&ret.ty));
        }
        out.push_str(" {\n");
        Self::emit_body(out, &ep.body);
        out.push_str("}\n\n");
    }
}

/// Lower WJSL if-expressions to WGSL `select(false_case, true_case, cond)`.
///
/// WGSL has if *statements* but not if *expressions*. WJSL allows Rust-style
/// `if (cond) { a } else { b }` as a value; we lower it during codegen.
fn lower_if_expressions(body: &str) -> String {
    let mut out = String::new();
    let mut i = 0;
    let bytes = body.as_bytes();

    while i < bytes.len() {
        if let Some(rel) = find_if_expression_at(&body[i..]) {
            let start = i + rel;
            if let Some((replacement, end)) = try_lower_if_expression(&body[start..]) {
                out.push_str(&body[i..start]);
                out.push_str(&replacement);
                i = start + end;
                continue;
            }
        }
        let ch = body[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// Find `if (` that begins an if-expression (not a statement-level if).
fn find_if_expression_at(s: &str) -> Option<usize> {
    let mut search_from = 0;
    while let Some(rel) = s[search_from..].find("if") {
        let pos = search_from + rel;
        if !s[pos..].starts_with("if (") && !s[pos..].starts_with("if(") {
            search_from = pos + 2;
            continue;
        }
        if is_if_expression_context(&s[..pos]) {
            return Some(pos);
        }
        search_from = pos + 2;
    }
    None
}

fn is_if_expression_context(prefix: &str) -> bool {
    let trimmed = prefix.trim_end();
    if trimmed.is_empty() {
        return false;
    }
    let last = trimmed.chars().last().unwrap();
    matches!(last, '=' | '(' | ',' | '+' | '-' | '*' | '/' | '%' | '>')
        || trimmed.ends_with("return")
}

fn try_lower_if_expression(s: &str) -> Option<(String, usize)> {
    let mut i = 0;
    if !s.starts_with("if") {
        return None;
    }
    i += 2;
    while i < s.len() && s.as_bytes()[i].is_ascii_whitespace() {
        i += 1;
    }
    if i >= s.len() || s.as_bytes()[i] != b'(' {
        return None;
    }
    let cond_start = i + 1;
    let cond_end = match_matching_paren(s, i)?;
    let cond = s[cond_start..cond_end - 1].trim();

    i = cond_end;
    while i < s.len() && s.as_bytes()[i].is_ascii_whitespace() {
        i += 1;
    }
    if i >= s.len() || s.as_bytes()[i] != b'{' {
        return None;
    }
    let then_start = i + 1;
    let then_end = match_matching_brace(s, i)?;
    let then_body = s[then_start..then_end - 1].trim();

    i = then_end;
    while i < s.len() && s.as_bytes()[i].is_ascii_whitespace() {
        i += 1;
    }
    if !s[i..].starts_with("else") {
        return None;
    }
    i += 4;
    while i < s.len() && s.as_bytes()[i].is_ascii_whitespace() {
        i += 1;
    }
    // `else if` chains are handled by recursive scan after lowering this segment
    if s[i..].starts_with("if") {
        return None;
    }
    if i >= s.len() || s.as_bytes()[i] != b'{' {
        return None;
    }
    let else_start = i + 1;
    let else_end = match_matching_brace(s, i)?;
    let else_body = s[else_start..else_end - 1].trim();

    let replacement = format!("select({else_body}, {then_body}, {cond})");
    Some((replacement, else_end))
}

fn match_matching_paren(s: &str, open_pos: usize) -> Option<usize> {
    if s.as_bytes().get(open_pos)? != &b'(' {
        return None;
    }
    let mut depth = 0;
    for (idx, ch) in s[open_pos..].char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open_pos + idx + 1);
                }
            }
            _ => {}
        }
    }
    None
}

fn match_matching_brace(s: &str, open_pos: usize) -> Option<usize> {
    if s.as_bytes().get(open_pos)? != &b'{' {
        return None;
    }
    let mut depth = 0;
    for (idx, ch) in s[open_pos..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open_pos + idx + 1);
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lower_inline_if_expression() {
        let body = "let panel_h = if (drill > 0.5) { 100u } else { 50u };";
        let out = lower_if_expressions(body);
        assert_eq!(out, "let panel_h = select(50u, 100u, drill > 0.5);");
    }

    #[test]
    fn test_lower_if_expression_in_call() {
        let body = "return pick(if (fps >= 60.0) { 0.5 } else { 1.1 });";
        let out = lower_if_expressions(body);
        assert_eq!(out, "return pick(select(1.1, 0.5, fps >= 60.0));");
    }

    #[test]
    fn test_statement_if_not_lowered() {
        let body = "if (a > 0) {\n    return 1.0;\n}";
        let out = lower_if_expressions(body);
        assert_eq!(out, body);
    }
}

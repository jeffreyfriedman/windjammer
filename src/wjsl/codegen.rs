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

        // 2b. Private variables
        for pv in &self.module.private_vars {
            out.push_str("var<private> ");
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
        out.push_str("(");
        for (i, p) in f.params.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            self.emit_param(out, p);
        }
        out.push_str(")");
        if let Some(ref ret) = f.return_type {
            out.push_str(" -> ");
            out.push_str(&self.type_to_wgsl(ret));
        }
        out.push_str(" {\n");
        if !f.body.is_empty() {
            for line in f.body.lines() {
                out.push_str("    ");
                out.push_str(line);
                out.push('\n');
            }
        }
        out.push_str("}\n\n");
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
        out.push_str("(");
        for (i, p) in ep.params.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            self.emit_param(out, p);
        }
        out.push_str(")");
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
        if !ep.body.is_empty() {
            for line in ep.body.lines() {
                out.push_str("    ");
                out.push_str(line);
                out.push('\n');
            }
        }
        out.push_str("}\n\n");
    }
}

//! Single WGSL printer for WJSL types, structs and bindings.
//!
//! Every WGSL emitter in the compiler (WJSL codegen, block-syntax shaders and
//! the Windjammer-source WGSL backend) prints types through this module.

use crate::wjsl::ast::*;

pub(crate) fn texture_type_to_wgsl(tt: &TextureType) -> String {
    match tt {
        TextureType::Texture2D(st) => format!("texture_2d<{}>", scalar_to_wgsl(*st)),
        TextureType::TextureCube(st) => format!("texture_cube<{}>", scalar_to_wgsl(*st)),
        TextureType::Texture3D(st) => format!("texture_3d<{}>", scalar_to_wgsl(*st)),
    }
}

pub(crate) fn scalar_to_wgsl(st: ScalarType) -> &'static str {
    match st {
        ScalarType::F32 => "f32",
        ScalarType::F64 => "f64",
        ScalarType::U32 => "u32",
        ScalarType::I32 => "i32",
        ScalarType::Bool => "bool",
    }
}

pub(crate) fn type_to_wgsl(ty: &Type) -> String {
    match ty {
        Type::Scalar(st) => scalar_to_wgsl(*st).to_string(),
        Type::Vec2(elem) => {
            let e = elem.unwrap_or(ScalarType::F32);
            format!("vec2<{}>", scalar_to_wgsl(e))
        }
        Type::Vec3(elem) => {
            let e = elem.unwrap_or(ScalarType::F32);
            format!("vec3<{}>", scalar_to_wgsl(e))
        }
        Type::Vec4(elem) => {
            let e = elem.unwrap_or(ScalarType::F32);
            format!("vec4<{}>", scalar_to_wgsl(e))
        }
        Type::Mat2x2(elem) => {
            let e = elem.unwrap_or(ScalarType::F32);
            format!("mat2x2<{}>", scalar_to_wgsl(e))
        }
        Type::Mat3x3(elem) => {
            let e = elem.unwrap_or(ScalarType::F32);
            format!("mat3x3<{}>", scalar_to_wgsl(e))
        }
        Type::Mat4x4(elem) => {
            let e = elem.unwrap_or(ScalarType::F32);
            format!("mat4x4<{}>", scalar_to_wgsl(e))
        }
        Type::Array(inner, size) => {
            if let Some(n) = size {
                format!("array<{}, {}>", type_to_wgsl(inner), n)
            } else {
                format!("array<{}>", type_to_wgsl(inner))
            }
        }
        Type::Atomic(st) => format!("atomic<{}>", scalar_to_wgsl(*st)),
        Type::Struct(name) => name.clone(),
        Type::Texture2D(st) => format!("texture_2d<{}>", scalar_to_wgsl(*st)),
        Type::TextureCube(st) => format!("texture_cube<{}>", scalar_to_wgsl(*st)),
        Type::Texture3D(st) => format!("texture_3d<{}>", scalar_to_wgsl(*st)),
        Type::Sampler => "sampler".to_string(),
        Type::SamplerComparison => "sampler_comparison".to_string(),
    }
}

pub(crate) fn emit_struct(out: &mut String, s: &StructDecl) {
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
        out.push_str(&type_to_wgsl(&f.ty));
        out.push_str(",\n");
    }
    out.push_str("}\n\n");
}

pub(crate) fn emit_binding(out: &mut String, b: &Binding) {
    out.push_str(&format!("@group({}) @binding({}) ", b.group, b.binding));
    match &b.kind {
        BindingKind::Uniform(ty) => {
            out.push_str("var<uniform> ");
            out.push_str(&b.name);
            out.push_str(": ");
            out.push_str(&type_to_wgsl(ty));
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
            out.push_str(&type_to_wgsl(ty));
        }
        BindingKind::Texture { texture_type } => {
            out.push_str("var ");
            out.push_str(&b.name);
            out.push_str(": ");
            out.push_str(&texture_type_to_wgsl(texture_type));
        }
        BindingKind::Sampler => {
            out.push_str("var ");
            out.push_str(&b.name);
            out.push_str(": sampler");
        }
    }
    out.push_str(";\n");
}

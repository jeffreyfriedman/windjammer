//! Block-syntax shaders: `shader Name { uniform x: Vec2<f32> storage y: array<f32> }`.
//!
//! A compact front end that lowers into the WJSL AST types and prints through
//! the shared [`wgsl_printer`]. Only declarations are modelled; the generated
//! compute entry point is a fixed stub (function bodies are not supported).

use crate::wjsl::ast::{ScalarType, StorageAccess, StructDecl, StructField, Type};
use crate::wjsl::wgsl_printer;
use crate::wjsl::{Binding, BindingKind};
use anyhow::{anyhow, bail, Result};

/// A parsed block-syntax shader.
#[derive(Debug, Clone, PartialEq)]
pub struct BlockShader {
    pub name: String,
    pub uniforms: Vec<(String, Type)>,
    pub storage: Vec<(String, Type, StorageAccess)>,
}

/// Lexical tokens: identifiers or single punctuation characters.
fn tokenize(source: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut chars = source.chars().peekable();
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
        } else if c == '/' {
            chars.next();
            if chars.peek() != Some(&'/') {
                break;
            }
            while chars.peek().is_some_and(|&c| c != '\n') {
                chars.next();
            }
        } else if c.is_alphabetic() || c == '_' {
            let mut ident = String::new();
            while let Some(&c) = chars.peek().filter(|c| c.is_alphanumeric() || **c == '_') {
                ident.push(c);
                chars.next();
            }
            tokens.push(ident);
        } else if ":<>{}(),@".contains(c) {
            tokens.push(c.to_string());
            chars.next();
        } else {
            break;
        }
    }
    tokens
}

struct Cursor {
    tokens: Vec<String>,
    pos: usize,
}

impl Cursor {
    fn peek(&self) -> Option<&str> {
        self.tokens.get(self.pos).map(String::as_str)
    }

    fn next(&mut self) -> Option<String> {
        let t = self.tokens.get(self.pos).cloned();
        self.pos += 1;
        t
    }

    fn expect(&mut self, want: &str) -> Result<()> {
        match self.next() {
            Some(t) if t == want => Ok(()),
            other => Err(anyhow!("Expected {want:?}, found {other:?}")),
        }
    }

    fn ident(&mut self, what: &str) -> Result<String> {
        match self.next() {
            Some(t)
                if t.chars()
                    .next()
                    .is_some_and(|c| c.is_alphabetic() || c == '_') =>
            {
                Ok(t)
            }
            other => Err(anyhow!("Expected {what}, found {other:?}")),
        }
    }

    fn scalar(&mut self) -> Result<ScalarType> {
        let s = self.ident("scalar type")?;
        Ok(match s.as_str() {
            "f32" => ScalarType::F32,
            "f64" => ScalarType::F64,
            "u32" => ScalarType::U32,
            "i32" => ScalarType::I32,
            "bool" => ScalarType::Bool,
            _ => bail!("Unknown scalar type: {s}"),
        })
    }

    fn vec_elem(&mut self) -> Result<ScalarType> {
        self.expect("<")?;
        let st = self.scalar()?;
        self.expect(">")?;
        Ok(st)
    }

    fn ty(&mut self) -> Result<Type> {
        let name = self.ident("type")?;
        Ok(match name.as_str() {
            "Vec2" => Type::Vec2(Some(self.vec_elem()?)),
            "Vec3" => Type::Vec3(Some(self.vec_elem()?)),
            "Vec4" => Type::Vec4(Some(self.vec_elem()?)),
            "Mat4" => Type::Mat4x4(None),
            "array" => {
                self.expect("<")?;
                let inner = self.ty()?;
                self.expect(">")?;
                Type::Array(Box::new(inner), None)
            }
            "f32" | "f64" | "u32" | "i32" | "bool" => {
                self.pos -= 1;
                Type::Scalar(self.scalar()?)
            }
            _ => Type::Struct(name),
        })
    }

    /// Skip an `@attr ...` run (entry-point syntax) up to the next declaration.
    fn skip_attribute_run(&mut self) {
        self.pos += 1;
        while !matches!(self.peek(), None | Some("}" | "uniform" | "storage")) {
            self.pos += 1;
        }
    }
}

/// Parse block-syntax source.
pub fn parse_block_shader(source: &str) -> Result<BlockShader> {
    let mut c = Cursor {
        tokens: tokenize(source),
        pos: 0,
    };
    c.expect("shader")?;
    let name = c.ident("shader name")?;
    c.expect("{")?;
    let (mut uniforms, mut storage) = (Vec::new(), Vec::new());
    loop {
        match c.peek() {
            Some("uniform") => {
                c.pos += 1;
                let n = c.ident("uniform name")?;
                c.expect(":")?;
                uniforms.push((n, c.ty()?));
            }
            Some("storage") => {
                c.pos += 1;
                let n = c.ident("storage name")?;
                c.expect(":")?;
                let ty = c.ty()?;
                let access = match c.peek() {
                    Some("read") => StorageAccess::Read,
                    Some("write") => StorageAccess::Write,
                    _ => StorageAccess::ReadWrite,
                };
                if access != StorageAccess::ReadWrite {
                    c.pos += 1;
                }
                storage.push((n, ty, access));
            }
            Some("@") => c.skip_attribute_run(),
            _ => break,
        }
    }
    c.expect("}")?;
    Ok(BlockShader {
        name,
        uniforms,
        storage,
    })
}

fn is_f64_vector(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Vec2(Some(ScalarType::F64))
            | Type::Vec3(Some(ScalarType::F64))
            | Type::Vec4(Some(ScalarType::F64))
    )
}

/// Reject uniform types WGSL cannot represent (f64 vectors).
pub fn check_block_shader(shader: &BlockShader) -> Result<()> {
    match shader.uniforms.iter().find(|(_, ty)| is_f64_vector(ty)) {
        Some((name, _)) => {
            bail!("WGSL does not support f64 - use f32 for uniforms (uniform {name})")
        }
        None => Ok(()),
    }
}

/// Check that a host-side type matches the shader's declared uniform type.
pub fn check_uniform_match(shader: &BlockShader, host_var: &str, host_type: &Type) -> Result<()> {
    let (_, ty) = shader
        .uniforms
        .iter()
        .find(|(n, _)| n == host_var)
        .ok_or_else(|| anyhow!("Uniform '{host_var}' not found in shader"))?;
    if ty != host_type {
        bail!(
            "Type mismatch at uniform {host_var}: expected {}, found {}",
            wgsl_printer::type_to_wgsl(ty),
            wgsl_printer::type_to_wgsl(host_type)
        );
    }
    if is_f64_vector(host_type) {
        bail!("WGSL does not support f64 - use f32 for uniforms (uniform {host_var})");
    }
    Ok(())
}

const ENTRY_STUB_HEAD: &str = concat!(
    "@compute @workgroup_size(8, 8)\n",
    "fn main(@builtin(global_invocation_id) id: vec3<u32>) {\n",
);
const SCREEN_SIZE_GUARD: &str = concat!(
    "    let width = u32(uniforms.screen_size.x);\n",
    "    let height = u32(uniforms.screen_size.y);\n",
    "    if (id.x >= width || id.y >= height) {\n",
    "        return;\n",
    "    }\n",
);

fn binding(group_slot: u32, name: &str, kind: BindingKind) -> Binding {
    Binding {
        group: 0,
        binding: group_slot,
        name: name.to_string(),
        kind,
    }
}

/// Print a parsed block shader as WGSL.
pub fn emit_block_shader(shader: &BlockShader) -> String {
    let mut out = String::new();
    let mut slot = 0;
    if !shader.uniforms.is_empty() {
        let fields = shader
            .uniforms
            .iter()
            .map(|(name, ty)| StructField {
                name: name.clone(),
                ty: ty.clone(),
                align: None,
                size: None,
                location: None,
                builtin: None,
            })
            .collect();
        wgsl_printer::emit_struct(
            &mut out,
            &StructDecl {
                name: "Uniforms".into(),
                fields,
            },
        );
        let ub = binding(
            slot,
            "uniforms",
            BindingKind::Uniform(Type::Struct("Uniforms".into())),
        );
        wgsl_printer::emit_binding(&mut out, &ub);
        slot += 1;
    }
    for (name, ty, access_mode) in &shader.storage {
        let kind = BindingKind::Storage {
            access_mode: *access_mode,
            ty: ty.clone(),
        };
        wgsl_printer::emit_binding(&mut out, &binding(slot, name, kind));
        slot += 1;
    }
    if slot > 0 {
        out.push('\n');
    }
    out.push_str(ENTRY_STUB_HEAD);
    if shader.uniforms.iter().any(|(n, _)| n == "screen_size") {
        out.push_str(SCREEN_SIZE_GUARD);
    }
    out.push_str("    // TODO: Add shader body from .wjsl functions\n}\n");
    out
}

/// Compile block-syntax source to WGSL (parse, check, print).
pub fn compile_legacy_shader(source: &str) -> Result<String> {
    let shader = parse_block_shader(source)?;
    check_block_shader(&shader)?;
    Ok(emit_block_shader(&shader))
}

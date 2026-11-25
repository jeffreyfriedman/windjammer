//! WJSL Type Checker - Compile-time validation
//!
//! Catches shader type errors before codegen: binary ops, bindings, function signatures.

use crate::wjsl::ast::*;
use crate::wjsl::lexer::{Lexer, Token};
use crate::wjsl::parser::parse_wjsl;
use anyhow::Result;
use std::collections::HashMap;

/// Type check WJSL source. Returns Ok(()) if valid, Err with message if invalid.
pub fn type_check_wjsl(source: &str) -> Result<()> {
    let ast =
        parse_wjsl(source).map_err(|e| crate::wjsl::diagnostics::enrich_shader_error(e, source))?;
    check(&ast, source).map_err(|e| crate::wjsl::diagnostics::enrich_shader_error(e, source))
}

/// Type check a parsed shader module
pub fn check(ast: &ShaderModule, _source: &str) -> Result<()> {
    let mut checker = TypeChecker::new(ast);
    checker.check_bindings_unique()?;
    checker.check_entry_points()?;
    checker.check_functions()?;
    Ok(())
}

/// Type checker with symbol table
struct TypeChecker<'a> {
    ast: &'a ShaderModule,
    /// (group, binding) -> binding name for duplicate detection
    binding_slots: HashMap<(u32, u32), String>,
    /// Struct name -> StructDecl for field lookup
    structs: HashMap<String, &'a StructDecl>,
}

impl<'a> TypeChecker<'a> {
    fn new(ast: &'a ShaderModule) -> Self {
        let structs = ast
            .structs
            .iter()
            .map(|s| (s.name.clone(), s))
            .collect::<HashMap<_, _>>();
        Self {
            ast,
            binding_slots: HashMap::new(),
            structs,
        }
    }

    fn check_bindings_unique(&mut self) -> Result<()> {
        for b in &self.ast.bindings {
            let key = (b.group, b.binding);
            if let Some(prev) = self.binding_slots.get(&key) {
                return Err(anyhow::anyhow!(
                    "Duplicate @binding({}) in @group({}): '{}' conflicts with '{}'",
                    b.binding,
                    b.group,
                    b.name,
                    prev
                ));
            }
            self.binding_slots.insert(key, b.name.clone());
        }
        Ok(())
    }

    fn check_entry_points(&self) -> Result<()> {
        for ep in &self.ast.entry_points {
            self.check_function_body(
                &ep.body,
                &ep.params,
                ep.return_type.as_ref(),
                &ep.name,
                true,
                ep.body_line,
            )?;
        }
        Ok(())
    }

    fn check_functions(&self) -> Result<()> {
        for f in &self.ast.functions {
            let return_type_wrapper = f.return_type.as_ref().map(|t| ReturnType {
                ty: t.clone(),
                location: None,
                builtin: None,
            });
            self.check_function_body(
                &f.body,
                &f.params,
                return_type_wrapper.as_ref(),
                &f.name,
                false,
                f.body_line,
            )?;
        }
        Ok(())
    }

    fn collect_function_signatures(&self) -> Vec<(String, Type)> {
        self.ast
            .functions
            .iter()
            .filter_map(|f| {
                f.return_type
                    .as_ref()
                    .map(|rt| (f.name.clone(), rt.clone()))
            })
            .collect()
    }

    fn check_function_body(
        &self,
        body: &str,
        params: &[Param],
        return_type: Option<&ReturnType>,
        fn_name: &str,
        _is_entry: bool,
        body_line: usize,
    ) -> Result<()> {
        let mut symbols = HashMap::new();
        for p in params {
            symbols.insert(p.name.clone(), p.ty.clone());
        }
        for b in &self.ast.bindings {
            let ty = match &b.kind {
                BindingKind::Uniform(t) => t.clone(),
                BindingKind::Storage { ty, .. } => ty.clone(),
                BindingKind::Texture { texture_type } => match texture_type {
                    TextureType::Texture2D(s) => Type::Texture2D(*s),
                    TextureType::TextureCube(s) => Type::TextureCube(*s),
                    TextureType::Texture3D(s) => Type::Texture3D(*s),
                },
                BindingKind::Sampler => Type::Sampler,
            };
            symbols.insert(b.name.clone(), ty);
        }
        for pv in &self.ast.private_vars {
            symbols.insert(pv.name.clone(), pv.ty.clone());
        }
        for cd in &self.ast.const_decls {
            if let Some(ref ty) = cd.ty {
                symbols.insert(cd.name.clone(), ty.clone());
            }
        }

        let fn_sigs = self.collect_function_signatures();
        let mut parser = BodyParser::new(body, symbols, self.structs.clone(), fn_sigs);
        parser.fn_names = self.ast.functions.iter().map(|f| f.name.clone()).collect();
        parser.body_line = body_line;
        parser.fn_name = fn_name.to_string();
        parser.parse_and_check(return_type)
    }
}

/// Parser for function body - extracts statements and type-checks expressions
pub(crate) struct BodyParser<'a> {
    pub(crate) lexer: Lexer<'a>,
    pub(crate) current: Token,
    pub(crate) current_line: usize,
    pub(crate) current_column: usize,
    pub(crate) symbols: HashMap<String, Type>,
    pub(crate) structs: HashMap<String, &'a StructDecl>,
    pub(crate) ast_functions: Vec<(String, Type)>,
    /// Every user function name (including void ones, which have no signature entry).
    pub(crate) fn_names: Vec<String>,
    /// 1-based source line where this body starts.
    pub(crate) body_line: usize,
    /// Name of the function whose body is being checked.
    pub(crate) fn_name: String,
}

impl<'a> BodyParser<'a> {
    pub(crate) fn new(
        body: &'a str,
        symbols: HashMap<String, Type>,
        structs: HashMap<String, &'a StructDecl>,
        ast_functions: Vec<(String, Type)>,
    ) -> Self {
        let mut lexer = Lexer::new(body);
        let line = lexer.line();
        let column = lexer.column();
        let current = lexer.next_token();
        Self {
            lexer,
            current,
            current_line: line,
            current_column: column,
            symbols,
            structs,
            ast_functions,
            fn_names: Vec::new(),
            body_line: 1,
            fn_name: String::new(),
        }
    }
}

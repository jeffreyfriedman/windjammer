//! Global Copy struct/enum discovery for multi-file library compilation.

use crate::lexer::Lexer;
use crate::parser::{Expression, Item, Parser};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Copy-shape check for library PASS 0.
/// Delegates to the canonical implementation in `type_classification`.
fn is_type_copy_quick_for_library(
    ty: &crate::parser::Type,
    copy_structs: &HashSet<String>,
    copy_enums: &HashSet<String>,
) -> bool {
    crate::type_classification::is_type_copy_with_registries(ty, copy_structs, copy_enums)
}

/// Discover Copy structs/enums across all library sources (including nested `mod` items).
/// Returns (copy_structs, all_local_struct_names, explicit_non_copy_structs).
pub(crate) fn collect_global_copy_structs_for_library_with_programs(
    sources: &[(PathBuf, String)],
    parsed_programs: Option<&[crate::parser::Program<'static>]>,
) -> (
    HashSet<String>,
    HashSet<String>,
    HashSet<String>,
    HashSet<String>,
) {
    use crate::parser::ast::EnumVariantData;

    struct StructInfo {
        name: String,
        field_types: Vec<crate::parser::Type>,
    }

    fn walk_items(
        items: &[Item<'_>],
        all_structs: &mut Vec<StructInfo>,
        global_copy_structs: &mut HashSet<String>,
        explicit_copy_structs: &mut HashSet<String>,
        copy_enums: &mut HashSet<String>,
        struct_names: &mut HashSet<String>,
        explicit_non_copy: &mut HashSet<String>,
    ) {
        for item in items {
            match item {
                Item::Struct { decl, .. } => {
                    let has_derive = decl.decorators.iter().any(|d| d.name == "derive");
                    let has_copy = decl.decorators.iter().any(|d| {
                        d.name == "derive"
                            && d.arguments.iter().any(|(_, arg)| {
                                matches!(arg, Expression::Identifier { name, .. } if name == "Copy")
                            })
                    });
                    let field_types: Vec<crate::parser::Type> =
                        decl.fields.iter().map(|f| f.field_type.clone()).collect();
                    struct_names.insert(decl.name.clone());
                    all_structs.push(StructInfo {
                        name: decl.name.clone(),
                        field_types,
                    });
                    if has_copy {
                        global_copy_structs.insert(decl.name.clone());
                        explicit_copy_structs.insert(decl.name.clone());
                    } else if has_derive {
                        // Struct has explicit @derive(...) without Copy — opt-out
                        explicit_non_copy.insert(decl.name.clone());
                    }
                }
                Item::Enum { decl, .. } => {
                    let is_unit_only = decl
                        .variants
                        .iter()
                        .all(|v| matches!(v.data, EnumVariantData::Unit));
                    if is_unit_only {
                        copy_enums.insert(decl.name.clone());
                    }
                }
                Item::Mod { items: inner, .. } => {
                    walk_items(
                        inner,
                        all_structs,
                        global_copy_structs,
                        explicit_copy_structs,
                        copy_enums,
                        struct_names,
                        explicit_non_copy,
                    );
                }
                _ => {}
            }
        }
    }

    let mut all_structs: Vec<StructInfo> = Vec::new();
    let mut global_copy_structs = HashSet::new();
    let mut explicit_copy_structs = HashSet::new();
    let mut copy_enums = HashSet::new();
    let mut struct_names = HashSet::new();
    let mut explicit_non_copy = HashSet::new();

    for (i, (file, source)) in sources.iter().enumerate() {
        let items = if let Some(programs) = parsed_programs {
            &programs[i].items
        } else {
            let mut lexer = Lexer::new(source);
            let tokens = lexer.tokenize_with_locations();
            let mut parser =
                Parser::new_with_source(tokens, file.to_string_lossy().to_string(), source.clone());
            let Ok(program) = parser.parse() else {
                eprintln!(
                    "Warning: Skipping file for Copy registry (parse error): {}",
                    file.display()
                );
                continue;
            };
            walk_items(
                &program.items,
                &mut all_structs,
                &mut global_copy_structs,
                &mut explicit_copy_structs,
                &mut copy_enums,
                &mut struct_names,
                &mut explicit_non_copy,
            );
            continue;
        };
        walk_items(
            items,
            &mut all_structs,
            &mut global_copy_structs,
            &mut explicit_copy_structs,
            &mut copy_enums,
            &mut struct_names,
            &mut explicit_non_copy,
        );
    }

    copy_enums.retain(|name| !struct_names.contains(name));

    let mut structs_by_name: HashMap<String, Vec<&StructInfo>> = HashMap::new();
    for s in &all_structs {
        structs_by_name.entry(s.name.clone()).or_default().push(s);
    }

    loop {
        let mut changed = false;
        for (name, variants) in &structs_by_name {
            if global_copy_structs.contains(name) || explicit_non_copy.contains(name) {
                continue;
            }

            let all_variants_copy = variants.iter().all(|s| {
                s.field_types.is_empty()
                    || s.field_types.iter().all(|ty| {
                        is_type_copy_quick_for_library(ty, &global_copy_structs, &copy_enums)
                    })
            });

            if all_variants_copy {
                global_copy_structs.insert(name.clone());
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    global_copy_structs.extend(copy_enums.iter().cloned());
    (
        global_copy_structs,
        struct_names,
        explicit_non_copy,
        explicit_copy_structs,
    )
}

/// Enums that must not be treated as Copy (data-carrying variants with non-Copy fields).
pub(crate) fn collect_non_copy_enums_for_library_with_programs(
    sources: &[(PathBuf, String)],
    parsed_programs: Option<&[crate::parser::Program<'static>]>,
) -> HashSet<String> {
    use crate::parser::ast::EnumVariantData;

    let (copy_structs, _, _, _) =
        collect_global_copy_structs_for_library_with_programs(sources, parsed_programs);

    fn variant_fields_copy(
        variant: &crate::parser::EnumVariant,
        copy_structs: &HashSet<String>,
    ) -> bool {
        let field_types: Vec<&crate::parser::Type> = match &variant.data {
            EnumVariantData::Unit => return true,
            EnumVariantData::Tuple(types) => types.iter().collect(),
            EnumVariantData::Struct(fields) => fields.iter().map(|(_, t)| t).collect(),
        };
        field_types
            .iter()
            .all(|t| is_type_copy_quick_for_library(t, copy_structs, &HashSet::new()))
    }

    fn walk_items(
        items: &[Item<'_>],
        non_copy: &mut HashSet<String>,
        copy_structs: &HashSet<String>,
    ) {
        for item in items {
            match item {
                Item::Enum { decl, .. } => {
                    if copy_structs.contains(&decl.name) {
                        continue;
                    }
                    let all_copy = decl
                        .variants
                        .iter()
                        .all(|v| variant_fields_copy(v, copy_structs));
                    if !all_copy {
                        non_copy.insert(decl.name.clone());
                    }
                }
                Item::Mod { items: inner, .. } => walk_items(inner, non_copy, copy_structs),
                _ => {}
            }
        }
    }

    let mut non_copy = HashSet::new();
    for (i, (file, source)) in sources.iter().enumerate() {
        let items = if let Some(programs) = parsed_programs {
            &programs[i].items
        } else {
            let mut lexer = Lexer::new(source);
            let tokens = lexer.tokenize_with_locations();
            let mut parser =
                Parser::new_with_source(tokens, file.to_string_lossy().to_string(), source.clone());
            let Ok(program) = parser.parse() else {
                eprintln!(
                    "Warning: Skipping file for non-Copy enum registry (parse error): {}",
                    file.display()
                );
                continue;
            };
            walk_items(&program.items, &mut non_copy, &copy_structs);
            continue;
        };
        walk_items(items, &mut non_copy, &copy_structs);
    }

    non_copy
}

/// Collect enum variant payload types from all parsed library programs.
/// Used for call-site string coercion on static factory helpers (`QuestReward::relationship`).
pub(crate) fn collect_global_enum_variant_types_for_library(
    parsed_programs: &[crate::parser::Program<'_>],
) -> HashMap<String, Vec<crate::parser::Type>> {
    use crate::parser::{EnumVariantData, Item, Statement};

    let mut map = HashMap::new();

    fn walk_enums(items: &[Item<'_>], map: &mut HashMap<String, Vec<crate::parser::Type>>) {
        for item in items {
            match item {
                Item::Enum { decl, .. } => {
                    for variant in &decl.variants {
                        let key = format!("{}::{}", decl.name, variant.name);
                        let types = match &variant.data {
                            EnumVariantData::Unit => Vec::new(),
                            EnumVariantData::Tuple(ts) => ts.clone(),
                            EnumVariantData::Struct(fields) => {
                                fields.iter().map(|(_, t)| t.clone()).collect()
                            }
                        };
                        map.insert(key, types);
                    }
                }
                Item::Mod { items: inner, .. } => walk_enums(inner, map),
                _ => {}
            }
        }
    }

    fn enum_ctor_from_call_callee(function: &Expression) -> Option<(String, String)> {
        if let Expression::FieldAccess { object, field, .. } = function {
            if let Expression::Identifier { name, .. } = object {
                return Some((name.clone(), field.clone()));
            }
        }
        if let Expression::Identifier { name, .. } = function {
            if let Some((enum_name, variant)) = name.rsplit_once("::") {
                return Some((enum_name.to_string(), variant.to_string()));
            }
        }
        None
    }

    fn call_uses_any_param(
        args: &[(Option<String>, &Expression)],
        param_names: &HashSet<String>,
    ) -> bool {
        args.iter().any(
            |(_, e)| matches!(e, Expression::Identifier { name, .. } if param_names.contains(name)),
        )
    }

    fn walk_expr_for_factory<'ast>(
        expr: &Expression<'ast>,
        param_names: &HashSet<String>,
        variant_types: &HashMap<String, Vec<crate::parser::Type>>,
        receiver_type: &str,
        method_name: &str,
        out: &mut HashMap<String, Vec<crate::parser::Type>>,
    ) {
        match expr {
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                if let Some((enum_name, variant)) = enum_ctor_from_call_callee(function) {
                    let variant_key = format!("{enum_name}::{variant}");
                    if call_uses_any_param(arguments, param_names) {
                        if let Some(types) = variant_types.get(&variant_key) {
                            out.insert(format!("{receiver_type}::{method_name}"), types.clone());
                        }
                    }
                }
                walk_expr_for_factory(
                    function,
                    param_names,
                    variant_types,
                    receiver_type,
                    method_name,
                    out,
                );
                for (_, arg) in arguments {
                    walk_expr_for_factory(
                        arg,
                        param_names,
                        variant_types,
                        receiver_type,
                        method_name,
                        out,
                    );
                }
            }
            Expression::StructLiteral { fields, .. } => {
                for (_, v) in fields {
                    walk_expr_for_factory(
                        v,
                        param_names,
                        variant_types,
                        receiver_type,
                        method_name,
                        out,
                    );
                }
            }
            Expression::MethodCall {
                object, arguments, ..
            } => {
                walk_expr_for_factory(
                    object,
                    param_names,
                    variant_types,
                    receiver_type,
                    method_name,
                    out,
                );
                for (_, arg) in arguments {
                    walk_expr_for_factory(
                        arg,
                        param_names,
                        variant_types,
                        receiver_type,
                        method_name,
                        out,
                    );
                }
            }
            Expression::Binary { left, right, .. } => {
                walk_expr_for_factory(
                    left,
                    param_names,
                    variant_types,
                    receiver_type,
                    method_name,
                    out,
                );
                walk_expr_for_factory(
                    right,
                    param_names,
                    variant_types,
                    receiver_type,
                    method_name,
                    out,
                );
            }
            Expression::Unary { operand, .. } => {
                walk_expr_for_factory(
                    operand,
                    param_names,
                    variant_types,
                    receiver_type,
                    method_name,
                    out,
                );
            }
            Expression::FieldAccess { object, .. } => {
                walk_expr_for_factory(
                    object,
                    param_names,
                    variant_types,
                    receiver_type,
                    method_name,
                    out,
                );
            }
            Expression::Cast { expr, .. } | Expression::TryOp { expr, .. } => {
                walk_expr_for_factory(
                    expr,
                    param_names,
                    variant_types,
                    receiver_type,
                    method_name,
                    out,
                );
            }
            Expression::Index { object, index, .. } => {
                walk_expr_for_factory(
                    object,
                    param_names,
                    variant_types,
                    receiver_type,
                    method_name,
                    out,
                );
                walk_expr_for_factory(
                    index,
                    param_names,
                    variant_types,
                    receiver_type,
                    method_name,
                    out,
                );
            }
            Expression::Tuple { elements, .. } | Expression::Array { elements, .. } => {
                for el in elements {
                    walk_expr_for_factory(
                        el,
                        param_names,
                        variant_types,
                        receiver_type,
                        method_name,
                        out,
                    );
                }
            }
            Expression::Block { statements, .. } => {
                walk_stmts_for_factory(
                    statements,
                    param_names,
                    variant_types,
                    receiver_type,
                    method_name,
                    out,
                );
            }
            _ => {}
        }
    }

    fn walk_stmts_for_factory<'ast>(
        stmts: &[&Statement<'ast>],
        param_names: &HashSet<String>,
        variant_types: &HashMap<String, Vec<crate::parser::Type>>,
        receiver_type: &str,
        method_name: &str,
        out: &mut HashMap<String, Vec<crate::parser::Type>>,
    ) {
        for stmt in stmts {
            match stmt {
                Statement::Return {
                    value: Some(expr), ..
                }
                | Statement::Expression { expr, .. } => {
                    walk_expr_for_factory(
                        expr,
                        param_names,
                        variant_types,
                        receiver_type,
                        method_name,
                        out,
                    );
                }
                Statement::Let {
                    value, else_block, ..
                } => {
                    walk_expr_for_factory(
                        value,
                        param_names,
                        variant_types,
                        receiver_type,
                        method_name,
                        out,
                    );
                    if let Some(stmts) = else_block {
                        walk_stmts_for_factory(
                            stmts,
                            param_names,
                            variant_types,
                            receiver_type,
                            method_name,
                            out,
                        );
                    }
                }
                Statement::Assignment { target, value, .. } => {
                    walk_expr_for_factory(
                        target,
                        param_names,
                        variant_types,
                        receiver_type,
                        method_name,
                        out,
                    );
                    walk_expr_for_factory(
                        value,
                        param_names,
                        variant_types,
                        receiver_type,
                        method_name,
                        out,
                    );
                }
                Statement::If {
                    then_block,
                    else_block,
                    condition,
                    ..
                } => {
                    walk_expr_for_factory(
                        condition,
                        param_names,
                        variant_types,
                        receiver_type,
                        method_name,
                        out,
                    );
                    walk_stmts_for_factory(
                        then_block,
                        param_names,
                        variant_types,
                        receiver_type,
                        method_name,
                        out,
                    );
                    if let Some(stmts) = else_block {
                        walk_stmts_for_factory(
                            stmts,
                            param_names,
                            variant_types,
                            receiver_type,
                            method_name,
                            out,
                        );
                    }
                }
                _ => {}
            }
        }
    }

    fn register_impl_factory_methods(
        items: &[Item<'_>],
        variant_types: &HashMap<String, Vec<crate::parser::Type>>,
        out: &mut HashMap<String, Vec<crate::parser::Type>>,
    ) {
        for item in items {
            match item {
                Item::Impl { block, .. } => {
                    for func in &block.functions {
                        let param_names: HashSet<String> =
                            func.parameters.iter().map(|p| p.name.clone()).collect();
                        walk_stmts_for_factory(
                            &func.body,
                            &param_names,
                            variant_types,
                            &block.type_name,
                            &func.name,
                            out,
                        );
                    }
                }
                Item::Mod { items: inner, .. } => {
                    register_impl_factory_methods(inner, variant_types, out);
                }
                _ => {}
            }
        }
    }

    for program in parsed_programs {
        walk_enums(&program.items, &mut map);
    }
    let variant_types = map.clone();
    for program in parsed_programs {
        register_impl_factory_methods(&program.items, &variant_types, &mut map);
    }

    map
}

/// Modules imported via `use std::…` in source text.
pub(crate) fn stdlib_modules_from_source(source: &str) -> HashSet<String> {
    let mut stdlib_modules = HashSet::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("use std::") {
            if let Some(module) = rest.split("::").next() {
                let module = module
                    .split(|c: char| c == ';' || c == ' ' || c == '{')
                    .next()
                    .unwrap_or(module);
                if !module.is_empty() {
                    stdlib_modules.insert(module.to_string());
                }
            }
        }
    }
    stdlib_modules
}

fn find_stdlib_dir_for_api_types() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("WINDJAMMER_STDLIB") {
        let pb = PathBuf::from(p);
        if pb.is_dir() {
            return Some(pb);
        }
    }
    let manifest_std = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("std");
    if manifest_std.is_dir() {
        return Some(manifest_std);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let dev = parent.parent()?.parent()?.join("std");
            if dev.is_dir() {
                return Some(dev);
            }
        }
    }
    let cwd = PathBuf::from("./std");
    if cwd.is_dir() {
        return Some(cwd);
    }
    None
}

/// Rust module path for a Windjammer `std::{module}` that maps to `windjammer_runtime`.
fn stdlib_module_rust_path(module: &str) -> Option<String> {
    match crate::codegen::rust::stdlib_method_traits::classify_wj_std_import(module) {
        crate::codegen::rust::stdlib_method_traits::WjStdImportKind::Runtime { rust_stem } => {
            Some(format!("windjammer_runtime::{rust_stem}"))
        }
        _ => None,
    }
}

/// Parse imported `std/{module}.wj` files and return struct field types + unit/tuple
/// enum variant payloads so single-file builds can coerce string literals into enum
/// fields (`method: "GET"` → `HttpMethod::GET`) without hardcoding names.
///
/// Also returns type-name → fully-qualified Rust path for FQ emit when the WJ
/// source only imports a sibling type (`ServerRequest` without `HttpMethod`).
pub(crate) fn collect_stdlib_api_types_for_modules(
    modules: &HashSet<String>,
) -> (
    HashMap<String, HashMap<String, crate::parser::Type>>,
    HashMap<String, Vec<crate::parser::Type>>,
    HashMap<String, String>,
) {
    use crate::parser::{EnumVariantData, Item};

    let mut struct_fields: HashMap<String, HashMap<String, crate::parser::Type>> = HashMap::new();
    let mut enum_variants: HashMap<String, Vec<crate::parser::Type>> = HashMap::new();
    let mut type_rust_paths: HashMap<String, String> = HashMap::new();
    let Some(stdlib_dir) = find_stdlib_dir_for_api_types() else {
        return (struct_fields, enum_variants, type_rust_paths);
    };

    fn walk_items(
        items: &[Item<'_>],
        rust_mod: Option<&str>,
        struct_fields: &mut HashMap<String, HashMap<String, crate::parser::Type>>,
        enum_variants: &mut HashMap<String, Vec<crate::parser::Type>>,
        type_rust_paths: &mut HashMap<String, String>,
    ) {
        for item in items {
            match item {
                Item::Struct { decl, .. } => {
                    let mut fields = HashMap::new();
                    for field in &decl.fields {
                        fields.insert(field.name.clone(), field.field_type.clone());
                    }
                    struct_fields.insert(decl.name.clone(), fields);
                    if let Some(rm) = rust_mod {
                        type_rust_paths.insert(decl.name.clone(), format!("{rm}::{}", decl.name));
                    }
                }
                Item::Enum { decl, .. } => {
                    if let Some(rm) = rust_mod {
                        type_rust_paths.insert(decl.name.clone(), format!("{rm}::{}", decl.name));
                    }
                    for variant in &decl.variants {
                        let key = format!("{}::{}", decl.name, variant.name);
                        let types = match &variant.data {
                            EnumVariantData::Unit => Vec::new(),
                            EnumVariantData::Tuple(ts) => ts.clone(),
                            EnumVariantData::Struct(fs) => {
                                fs.iter().map(|(_, t)| t.clone()).collect()
                            }
                        };
                        enum_variants.insert(key, types);
                    }
                }
                Item::Mod { items: inner, .. } => {
                    walk_items(
                        inner,
                        rust_mod,
                        struct_fields,
                        enum_variants,
                        type_rust_paths,
                    );
                }
                _ => {}
            }
        }
    }

    for module in modules {
        // Skip pure Rust-passthrough modules that have no Windjammer API surface here.
        if matches!(module.as_str(), "collections" | "cmp" | "ops" | "process") {
            continue;
        }
        let path = stdlib_dir.join(format!("{module}.wj"));
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        let mut lexer = Lexer::new(&source);
        let tokens = lexer.tokenize_with_locations();
        let mut parser = Parser::new(tokens);
        let Ok(program) = parser.parse() else {
            continue;
        };
        let rust_mod = stdlib_module_rust_path(module);
        walk_items(
            &program.items,
            rust_mod.as_deref(),
            &mut struct_fields,
            &mut enum_variants,
            &mut type_rust_paths,
        );
    }

    (struct_fields, enum_variants, type_rust_paths)
}

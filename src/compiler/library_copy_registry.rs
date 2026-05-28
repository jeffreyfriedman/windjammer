//! Global Copy struct/enum discovery for multi-file library compilation.

use crate::lexer::Lexer;
use crate::parser::{Expression, Item, Parser};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Copy-shape check for library PASS 0 (mirrors `main.rs` `is_type_copy_quick`).
fn is_type_copy_quick_for_library(
    ty: &crate::parser::Type,
    copy_structs: &HashSet<String>,
    copy_enums: &HashSet<String>,
) -> bool {
    use crate::parser::Type;
    match ty {
        Type::Int | Type::Int32 | Type::Uint | Type::Float | Type::Bool => true,
        Type::Reference(_) => true,
        Type::MutableReference(_) => false,
        Type::Tuple(types) => types
            .iter()
            .all(|t| is_type_copy_quick_for_library(t, copy_structs, copy_enums)),
        Type::Option(inner) => is_type_copy_quick_for_library(inner, copy_structs, copy_enums),
        Type::Result(ok, err) => {
            is_type_copy_quick_for_library(ok, copy_structs, copy_enums)
                && is_type_copy_quick_for_library(err, copy_structs, copy_enums)
        }
        Type::Array(inner, _) => is_type_copy_quick_for_library(inner, copy_structs, copy_enums),
        Type::Vec(_) | Type::String => false,
        Type::RawPointer { pointee, .. } => {
            is_type_copy_quick_for_library(pointee.as_ref(), copy_structs, copy_enums)
        }
        Type::FunctionPointer { .. } => true,
        Type::Custom(name) => {
            copy_structs.contains(name)
                || copy_enums.contains(name)
                || crate::type_classification::is_copy_primitive(name)
        }
        _ => false,
    }
}

/// Discover Copy structs/enums across all library sources (including nested `mod` items).
/// Returns (copy_structs, all_local_struct_names).
pub(crate) fn collect_global_copy_structs_for_library(
    sources: &[(PathBuf, String)],
) -> (HashSet<String>, HashSet<String>) {
    use crate::parser::ast::EnumVariantData;

    struct StructInfo {
        name: String,
        field_types: Vec<crate::parser::Type>,
    }

    fn walk_items(
        items: &[Item<'_>],
        all_structs: &mut Vec<StructInfo>,
        global_copy_structs: &mut HashSet<String>,
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
    let mut copy_enums = HashSet::new();
    let mut struct_names = HashSet::new();
    let mut explicit_non_copy = HashSet::new();

    for (file, source) in sources {
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
    (global_copy_structs, struct_names)
}

/// Enums that must not be treated as Copy (data-carrying variants with non-Copy fields).
pub(crate) fn collect_non_copy_enums_for_library(
    sources: &[(PathBuf, String)],
) -> HashSet<String> {
    use crate::parser::ast::EnumVariantData;

    let (copy_structs, _) = collect_global_copy_structs_for_library(sources);

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
    for (file, source) in sources {
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
    }

    non_copy
}

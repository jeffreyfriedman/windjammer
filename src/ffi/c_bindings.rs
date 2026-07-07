//! Generate C header declarations from IDL modules (WJ-IMPL-02).

use super::idl::{IdlFunction, IdlModule, IdlType, IdlTypeKind};

/// Generate a complete C header for the given IDL module.
pub fn generate_c_header(module: &IdlModule) -> String {
    let guard = header_guard_name(&module.name);
    let mut out = String::new();

    out.push_str(&format!("#ifndef {guard}\n"));
    out.push_str(&format!("#define {guard}\n\n"));
    out.push_str("#include <stdint.h>\n");
    out.push_str("#include <stddef.h>\n\n");

    if !module.types.is_empty() {
        out.push_str("/* Types */\n");
        for ty in &module.types {
            emit_type(&mut out, ty);
            out.push('\n');
        }
    }

    if !module.functions.is_empty() {
        out.push_str("/* Functions */\n");
        for func in &module.functions {
            emit_function(&mut out, func);
            out.push('\n');
        }
    }

    out.push_str(&format!("#endif /* {guard} */\n"));
    out
}

fn header_guard_name(module_name: &str) -> String {
    let sanitized: String = module_name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect();
    format!("WINDJAMMER_{sanitized}_H")
}

fn emit_type(out: &mut String, ty: &IdlType) {
    match ty.kind {
        IdlTypeKind::Opaque => {
            out.push_str(&format!("typedef struct {} {};\n", ty.name, ty.name));
        }
        IdlTypeKind::Enum => {
            out.push_str(&format!("typedef enum {} {{\n", ty.name));
            for (idx, field) in ty.fields.iter().enumerate() {
                let comma = if idx + 1 == ty.fields.len() { "" } else { "," };
                out.push_str(&format!(
                    "    {}_{}{}\n",
                    ty.name.to_ascii_uppercase(),
                    field.name.to_ascii_uppercase(),
                    comma
                ));
            }
            out.push_str(&format!("}} {};\n", ty.name));
        }
        IdlTypeKind::Struct => {
            out.push_str(&format!("typedef struct {} {{\n", ty.name));
            for field in &ty.fields {
                let c_type = map_type_to_c(&field.type_name);
                out.push_str(&format!("    {} {};\n", c_type, field.name));
            }
            out.push_str(&format!("}} {};\n", ty.name));
        }
    }
}

fn emit_function(out: &mut String, func: &IdlFunction) {
    if func.is_async {
        out.push_str("/* async: C binding uses callback or poll API */\n");
    }

    let return_type = func
        .return_type
        .as_deref()
        .map(map_type_to_c)
        .unwrap_or_else(|| "void".to_string());

    let params: Vec<String> = func
        .params
        .iter()
        .map(|p| format!("{} {}", map_type_to_c(&p.type_name), p.name))
        .collect();

    let param_list = if params.is_empty() {
        "void".to_string()
    } else {
        params.join(", ")
    };

    out.push_str(&format!(
        "extern {} {}({});\n",
        return_type, func.name, param_list
    ));
}

fn map_type_to_c(type_name: &str) -> String {
    match type_name {
        "i32" | "int" | "int32_t" => "int32_t".to_string(),
        "u32" | "uint32_t" => "uint32_t".to_string(),
        "i64" | "int64_t" => "int64_t".to_string(),
        "u64" | "uint64_t" => "uint64_t".to_string(),
        "f32" | "float" => "float".to_string(),
        "f64" | "double" => "double".to_string(),
        "bool" => "bool".to_string(),
        "string" | "String" | "str" => "const char*".to_string(),
        "usize" | "size_t" => "size_t".to_string(),
        "void" | "()" => "void".to_string(),
        other if other.starts_with("const char") => other.to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ffi::idl::{IdlField, IdlFunction, IdlParam, IdlType, IdlTypeKind};

    #[test]
    fn generate_c_header_matches_expected_shape() {
        let module = IdlModule {
            name: "module".to_string(),
            types: vec![IdlType {
                name: "MyStruct".to_string(),
                kind: IdlTypeKind::Struct,
                fields: vec![
                    IdlField {
                        name: "x".to_string(),
                        type_name: "int32_t".to_string(),
                    },
                    IdlField {
                        name: "y".to_string(),
                        type_name: "float".to_string(),
                    },
                ],
            }],
            functions: vec![IdlFunction {
                name: "my_function".to_string(),
                params: vec![
                    IdlParam {
                        name: "param1".to_string(),
                        type_name: "int32_t".to_string(),
                    },
                    IdlParam {
                        name: "param2".to_string(),
                        type_name: "const char*".to_string(),
                    },
                ],
                return_type: Some("MyStruct".to_string()),
                is_async: false,
            }],
        };

        let header = generate_c_header(&module);
        assert!(header.contains("#ifndef WINDJAMMER_MODULE_H"));
        assert!(header.contains("typedef struct MyStruct"));
        assert!(header.contains("int32_t x;"));
        assert!(header.contains("float y;"));
        assert!(header.contains("extern MyStruct my_function(int32_t param1, const char* param2);"));
        assert!(header.contains("#endif"));
    }
}

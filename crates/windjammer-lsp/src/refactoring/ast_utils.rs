//! Utilities for AST manipulation and text generation
#![allow(dead_code)] // Utility functions used by various refactoring modules

use tower_lsp::lsp_types::Position;
use tower_lsp::lsp_types::*;
use windjammer::parser::{Parameter, Type};

/// Convert LSP Range to line/column offsets
pub fn range_to_offsets(text: &str, range: Range) -> Option<(usize, usize)> {
    let lines: Vec<&str> = text.lines().collect();

    let start_line = range.start.line as usize;
    let start_col = range.start.character as usize;
    let end_line = range.end.line as usize;
    let end_col = range.end.character as usize;

    if start_line >= lines.len() || end_line >= lines.len() {
        return None;
    }

    // Calculate byte offset for start
    let mut start_offset = 0;
    for (i, line) in lines.iter().enumerate() {
        if i < start_line {
            start_offset += line.len() + 1; // +1 for newline
        } else if i == start_line {
            start_offset += start_col;
            break;
        }
    }

    // Calculate byte offset for end
    let mut end_offset = 0;
    for (i, line) in lines.iter().enumerate() {
        if i < end_line {
            end_offset += line.len() + 1;
        } else if i == end_line {
            end_offset += end_col;
            break;
        }
    }

    Some((start_offset, end_offset))
}

/// Extract text from a range
pub fn extract_text(source: &str, range: Range) -> Option<String> {
    let (start, end) = range_to_offsets(source, range)?;
    Some(source.get(start..end)?.to_string())
}

/// Generate function signature from parameters and return type
pub fn generate_function_signature(
    name: &str,
    parameters: &[Parameter],
    return_type: &Option<Type>,
) -> String {
    let mut sig = format!("fn {}", name);

    // Parameters
    sig.push('(');
    for (i, param) in parameters.iter().enumerate() {
        if i > 0 {
            sig.push_str(", ");
        }
        sig.push_str(&param.name);
        sig.push_str(": ");
        sig.push_str(&format_type(&param.type_));
    }
    sig.push(')');

    // Return type
    if let Some(ret_type) = return_type {
        sig.push_str(" -> ");
        sig.push_str(&format_type(ret_type));
    }

    sig
}

/// Format a type for code generation
pub fn format_type(ty: &Type) -> String {
    match ty {
        Type::Int => "int".to_string(),
        Type::Int32 => "i32".to_string(),
        Type::Uint => "uint".to_string(),
        Type::Float => "float".to_string(),
        Type::Bool => "bool".to_string(),
        Type::String => "string".to_string(),
        Type::Custom(name) => name.clone(),
        Type::Generic(name) => name.clone(),
        Type::Parameterized(base, args) => {
            let args_str = args.iter().map(format_type).collect::<Vec<_>>().join(", ");
            format!("{}<{}>", base, args_str)
        }
        Type::Associated(base, assoc) => format!("{}::{}", base, assoc),
        Type::TraitObject(name) => format!("dyn {}", name),
        Type::Option(inner) => format!("Option<{}>", format_type(inner)),
        Type::Result(ok, err) => format!("Result<{}, {}>", format_type(ok), format_type(err)),
        Type::Vec(inner) => format!("Vec<{}>", format_type(inner)),
        Type::Array(inner, size) => format!("[{}; {}]", format_type(inner), size),
        Type::Reference(inner) => format!("&{}", format_type(inner)),
        Type::MutableReference(inner) => format!("&mut {}", format_type(inner)),
        Type::Tuple(types) => {
            let types_str = types.iter().map(format_type).collect::<Vec<_>>().join(", ");
            format!("({})", types_str)
        }
    }
}

/// Generate a complete function declaration
pub fn generate_function(
    name: &str,
    parameters: &[Parameter],
    return_type: &Option<Type>,
    body: &str,
    indent_level: usize,
) -> String {
    let indent = "    ".repeat(indent_level);
    let mut result = String::new();

    // Function signature
    result.push_str(&indent);
    result.push_str(&generate_function_signature(name, parameters, return_type));
    result.push_str(" {\n");

    // Body (already indented from source)
    for line in body.lines() {
        if !line.trim().is_empty() {
            result.push_str(&indent);
            result.push_str("    ");
            result.push_str(line);
        }
        result.push('\n');
    }

    // Closing brace
    result.push_str(&indent);
    result.push('}');

    result
}

/// Generate a function call expression
pub fn generate_function_call(name: &str, arguments: &[String]) -> String {
    let args_str = arguments.join(", ");
    format!("{}({})", name, args_str)
}

/// Calculate indentation level from source text
pub fn get_indent_level(line: &str) -> usize {
    line.chars().take_while(|c| c.is_whitespace()).count() / 4
}

/// Get the indentation string for a line
pub fn get_indentation(line: &str) -> String {
    line.chars().take_while(|c| c.is_whitespace()).collect()
}

/// Convert LSP Position to byte offset in source text
pub fn position_to_byte_offset(source: &str, position: Position) -> usize {
    let mut byte_offset = 0;
    let mut current_line = 0;
    let mut current_char = 0;

    for ch in source.chars() {
        if current_line == position.line as usize && current_char == position.character as usize {
            return byte_offset;
        }

        if ch == '\n' {
            current_line += 1;
            current_char = 0;
        } else {
            current_char += 1;
        }

        byte_offset += ch.len_utf8();
    }

    // If we reached the end, return the last position
    byte_offset
}

/// Convert byte offset to LSP Position
pub fn byte_offset_to_position(source: &str, byte_offset: usize) -> Position {
    let mut line = 0;
    let mut character = 0;
    let mut current_offset = 0;

    for ch in source.chars() {
        if current_offset >= byte_offset {
            break;
        }

        if ch == '\n' {
            line += 1;
            character = 0;
        } else {
            character += 1;
        }

        current_offset += ch.len_utf8();
    }

    Position {
        line: line as u32,
        character: character as u32,
    }
}

/// Apply indentation to a multi-line string
pub fn indent_text(text: &str, indent_level: usize) -> String {
    let indent = "    ".repeat(indent_level);
    text.lines()
        .map(|line| {
            if line.trim().is_empty() {
                String::new()
            } else {
                format!("{}{}", indent, line)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Find the position to insert a new function (before the current function)
pub fn find_function_insertion_point(source: &str, current_fn_line: usize) -> usize {
    // Find the start of the current function
    let lines: Vec<&str> = source.lines().collect();

    // Go backwards from current line to find the start of the function
    let mut fn_start_line = current_fn_line;
    for (i, line) in lines.iter().enumerate().take(current_fn_line).rev() {
        if line.trim().starts_with("fn ") || line.trim().starts_with("pub fn ") {
            fn_start_line = i;
            break;
        }
    }

    // Insert before the function, accounting for decorators and comments
    let mut insert_line = fn_start_line;
    while insert_line > 0 {
        let line = lines[insert_line - 1].trim();
        if line.starts_with("//") || line.starts_with("#[") || line.starts_with("@") {
            insert_line -= 1;
        } else {
            break;
        }
    }

    insert_line
}

#[cfg(test)]
mod tests {
    use super::*;
    use windjammer::parser::Type;

    #[test]
    fn test_format_simple_type() {
        assert_eq!(format_type(&Type::Int), "int");
        assert_eq!(format_type(&Type::String), "string");
        assert_eq!(format_type(&Type::Custom("MyType".to_string())), "MyType");
    }

    #[test]
    fn test_format_generic_type() {
        let vec_int = Type::Vec(Box::new(Type::Int));
        assert_eq!(format_type(&vec_int), "Vec<int>");

        let option_str = Type::Option(Box::new(Type::String));
        assert_eq!(format_type(&option_str), "Option<string>");
    }

    #[test]
    fn test_generate_function_signature() {
        let params = vec![
            Parameter {
                name: "x".to_string(),
                pattern: None,
                type_: Type::Custom("int".to_string()),
                ownership: windjammer::parser::OwnershipHint::Inferred,
            },
            Parameter {
                name: "y".to_string(),
                pattern: None,
                type_: Type::Custom("int".to_string()),
                ownership: windjammer::parser::OwnershipHint::Inferred,
            },
        ];

        let return_type = Some(Type::Custom("int".to_string()));

        let sig = generate_function_signature("add", &params, &return_type);
        assert_eq!(sig, "fn add(x: int, y: int) -> int");
    }

    #[test]
    fn test_generate_function_call() {
        let call = generate_function_call("calculate", &["x".to_string(), "y".to_string()]);
        assert_eq!(call, "calculate(x, y)");
    }

    #[test]
    fn test_get_indent_level() {
        assert_eq!(get_indent_level("no indent"), 0);
        assert_eq!(get_indent_level("    one level"), 1);
        assert_eq!(get_indent_level("        two levels"), 2);
    }

    #[test]
    fn test_indent_text() {
        let text = "line1\nline2\nline3";
        let indented = indent_text(text, 1);
        assert_eq!(indented, "    line1\n    line2\n    line3");
    }
}

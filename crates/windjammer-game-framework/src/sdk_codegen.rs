//! # SDK Code Generation Framework
//!
//! Generates SDK code in multiple languages from IDL definitions.
//!
//! ## Features
//! - Multi-language code generation
//! - Template-based generation
//! - Type mapping between languages
//! - Documentation generation
//! - Package structure generation
//!
//! ## Example
//! ```no_run
//! use windjammer_game_framework::sdk_codegen::{CodeGenerator, Language};
//! use windjammer_game_framework::sdk_idl::ApiDefinition;
//!
//! let api = ApiDefinition::new("windjammer", "1.0.0");
//! let generator = CodeGenerator::new(Language::Python);
//! let code = generator.generate(&api).unwrap();
//! ```

use crate::sdk_idl::*;
use std::collections::HashMap;
use std::fmt;

/// Supported target languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    CSharp,
    Cpp,
    Go,
    Java,
    Lua,
    Swift,
    Ruby,
}

impl Language {
    /// Get file extension for this language
    pub fn extension(&self) -> &str {
        match self {
            Language::Rust => "rs",
            Language::Python => "py",
            Language::JavaScript => "js",
            Language::TypeScript => "ts",
            Language::CSharp => "cs",
            Language::Cpp => "cpp",
            Language::Go => "go",
            Language::Java => "java",
            Language::Lua => "lua",
            Language::Swift => "swift",
            Language::Ruby => "rb",
        }
    }

    /// Get package manager for this language
    pub fn package_manager(&self) -> &str {
        match self {
            Language::Rust => "cargo",
            Language::Python => "pip",
            Language::JavaScript | Language::TypeScript => "npm",
            Language::CSharp => "nuget",
            Language::Cpp => "conan",
            Language::Go => "go",
            Language::Java => "maven",
            Language::Lua => "luarocks",
            Language::Swift => "swift",
            Language::Ruby => "gem",
        }
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Language::Rust => write!(f, "Rust"),
            Language::Python => write!(f, "Python"),
            Language::JavaScript => write!(f, "JavaScript"),
            Language::TypeScript => write!(f, "TypeScript"),
            Language::CSharp => write!(f, "C#"),
            Language::Cpp => write!(f, "C++"),
            Language::Go => write!(f, "Go"),
            Language::Java => write!(f, "Java"),
            Language::Lua => write!(f, "Lua"),
            Language::Swift => write!(f, "Swift"),
            Language::Ruby => write!(f, "Ruby"),
        }
    }
}

/// Code generation error
#[derive(Debug, Clone)]
pub enum CodeGenError {
    /// Unsupported type
    UnsupportedType(String),
    /// Invalid definition
    InvalidDefinition(String),
    /// Template error
    TemplateError(String),
}

impl fmt::Display for CodeGenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CodeGenError::UnsupportedType(t) => write!(f, "Unsupported type: {}", t),
            CodeGenError::InvalidDefinition(d) => write!(f, "Invalid definition: {}", d),
            CodeGenError::TemplateError(e) => write!(f, "Template error: {}", e),
        }
    }
}

impl std::error::Error for CodeGenError {}

/// Generated code result
#[derive(Debug, Clone)]
pub struct GeneratedCode {
    /// Generated files (filename -> content)
    pub files: HashMap<String, String>,
    /// Package metadata
    pub metadata: PackageMetadata,
}

/// Package metadata
#[derive(Debug, Clone)]
pub struct PackageMetadata {
    /// Package name
    pub name: String,
    /// Package version
    pub version: String,
    /// Package description
    pub description: String,
    /// Package author
    pub author: String,
    /// Package license
    pub license: String,
    /// Dependencies
    pub dependencies: Vec<String>,
}

impl PackageMetadata {
    /// Create new package metadata
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            description: String::new(),
            author: String::new(),
            license: "MIT".to_string(),
            dependencies: Vec::new(),
        }
    }
}

/// Code generator
pub struct CodeGenerator {
    /// Target language
    language: Language,
    /// Type mappings
    type_map: HashMap<String, String>,
}

impl CodeGenerator {
    /// Create a new code generator
    pub fn new(language: Language) -> Self {
        let mut generator = Self {
            language,
            type_map: HashMap::new(),
        };
        generator.init_type_map();
        generator
    }

    /// Initialize type mappings for the target language
    fn init_type_map(&mut self) {
        match self.language {
            Language::Rust => {
                self.type_map.insert("bool".to_string(), "bool".to_string());
                self.type_map.insert("int32".to_string(), "i32".to_string());
                self.type_map.insert("int64".to_string(), "i64".to_string());
                self.type_map.insert("float32".to_string(), "f32".to_string());
                self.type_map.insert("float64".to_string(), "f64".to_string());
                self.type_map.insert("string".to_string(), "String".to_string());
            }
            Language::Python => {
                self.type_map.insert("bool".to_string(), "bool".to_string());
                self.type_map.insert("int32".to_string(), "int".to_string());
                self.type_map.insert("int64".to_string(), "int".to_string());
                self.type_map.insert("float32".to_string(), "float".to_string());
                self.type_map.insert("float64".to_string(), "float".to_string());
                self.type_map.insert("string".to_string(), "str".to_string());
            }
            Language::JavaScript | Language::TypeScript => {
                self.type_map.insert("bool".to_string(), "boolean".to_string());
                self.type_map.insert("int32".to_string(), "number".to_string());
                self.type_map.insert("int64".to_string(), "number".to_string());
                self.type_map.insert("float32".to_string(), "number".to_string());
                self.type_map.insert("float64".to_string(), "number".to_string());
                self.type_map.insert("string".to_string(), "string".to_string());
            }
            Language::CSharp => {
                self.type_map.insert("bool".to_string(), "bool".to_string());
                self.type_map.insert("int32".to_string(), "int".to_string());
                self.type_map.insert("int64".to_string(), "long".to_string());
                self.type_map.insert("float32".to_string(), "float".to_string());
                self.type_map.insert("float64".to_string(), "double".to_string());
                self.type_map.insert("string".to_string(), "string".to_string());
            }
            Language::Cpp => {
                self.type_map.insert("bool".to_string(), "bool".to_string());
                self.type_map.insert("int32".to_string(), "int32_t".to_string());
                self.type_map.insert("int64".to_string(), "int64_t".to_string());
                self.type_map.insert("float32".to_string(), "float".to_string());
                self.type_map.insert("float64".to_string(), "double".to_string());
                self.type_map.insert("string".to_string(), "std::string".to_string());
            }
            Language::Go => {
                self.type_map.insert("bool".to_string(), "bool".to_string());
                self.type_map.insert("int32".to_string(), "int32".to_string());
                self.type_map.insert("int64".to_string(), "int64".to_string());
                self.type_map.insert("float32".to_string(), "float32".to_string());
                self.type_map.insert("float64".to_string(), "float64".to_string());
                self.type_map.insert("string".to_string(), "string".to_string());
            }
            Language::Java => {
                self.type_map.insert("bool".to_string(), "boolean".to_string());
                self.type_map.insert("int32".to_string(), "int".to_string());
                self.type_map.insert("int64".to_string(), "long".to_string());
                self.type_map.insert("float32".to_string(), "float".to_string());
                self.type_map.insert("float64".to_string(), "double".to_string());
                self.type_map.insert("string".to_string(), "String".to_string());
            }
            Language::Lua => {
                // Lua is dynamically typed
                self.type_map.insert("bool".to_string(), "boolean".to_string());
                self.type_map.insert("int32".to_string(), "number".to_string());
                self.type_map.insert("int64".to_string(), "number".to_string());
                self.type_map.insert("float32".to_string(), "number".to_string());
                self.type_map.insert("float64".to_string(), "number".to_string());
                self.type_map.insert("string".to_string(), "string".to_string());
            }
            Language::Swift => {
                self.type_map.insert("bool".to_string(), "Bool".to_string());
                self.type_map.insert("int32".to_string(), "Int32".to_string());
                self.type_map.insert("int64".to_string(), "Int64".to_string());
                self.type_map.insert("float32".to_string(), "Float".to_string());
                self.type_map.insert("float64".to_string(), "Double".to_string());
                self.type_map.insert("string".to_string(), "String".to_string());
            }
            Language::Ruby => {
                // Ruby is dynamically typed
                self.type_map.insert("bool".to_string(), "Boolean".to_string());
                self.type_map.insert("int32".to_string(), "Integer".to_string());
                self.type_map.insert("int64".to_string(), "Integer".to_string());
                self.type_map.insert("float32".to_string(), "Float".to_string());
                self.type_map.insert("float64".to_string(), "Float".to_string());
                self.type_map.insert("string".to_string(), "String".to_string());
            }
        }
    }

    /// Generate SDK code from API definition
    pub fn generate(&self, api: &ApiDefinition) -> Result<GeneratedCode, CodeGenError> {
        let mut files = HashMap::new();

        // Generate main module file
        let main_file = self.generate_main_module(api)?;
        files.insert(format!("main.{}", self.language.extension()), main_file);

        // Generate struct files
        for struct_def in &api.structs {
            let struct_code = self.generate_struct(struct_def)?;
            files.insert(
                format!("{}.{}", struct_def.name.to_lowercase(), self.language.extension()),
                struct_code,
            );
        }

        // Generate enum files
        for enum_def in &api.enums {
            let enum_code = self.generate_enum(enum_def)?;
            files.insert(
                format!("{}.{}", enum_def.name.to_lowercase(), self.language.extension()),
                enum_code,
            );
        }

        // Generate class files
        for class_def in &api.classes {
            let class_code = self.generate_class(class_def)?;
            files.insert(
                format!("{}.{}", class_def.name.to_lowercase(), self.language.extension()),
                class_code,
            );
        }

        // Generate package metadata
        let metadata = PackageMetadata::new(&api.name, &api.version);

        Ok(GeneratedCode { files, metadata })
    }

    /// Generate main module file
    fn generate_main_module(&self, api: &ApiDefinition) -> Result<String, CodeGenError> {
        let mut code = String::new();

        // Add header comment
        code.push_str(&format!("// {} SDK v{}\n", api.name, api.version));
        code.push_str("// Auto-generated by Windjammer SDK Code Generator\n\n");

        // Generate functions
        for func in &api.functions {
            code.push_str(&self.generate_function(func)?);
            code.push('\n');
        }

        Ok(code)
    }

    /// Generate struct code
    fn generate_struct(&self, struct_def: &StructDef) -> Result<String, CodeGenError> {
        let mut code = String::new();

        match self.language {
            Language::Rust => {
                if !struct_def.doc.is_empty() {
                    code.push_str(&format!("/// {}\n", struct_def.doc));
                }
                code.push_str(&format!("pub struct {} {{\n", struct_def.name));
                for field in &struct_def.fields {
                    if !field.doc.is_empty() {
                        code.push_str(&format!("    /// {}\n", field.doc));
                    }
                    let field_type = self.map_type(&field.field_type)?;
                    code.push_str(&format!("    pub {}: {},\n", field.name, field_type));
                }
                code.push_str("}\n");
            }
            Language::Python => {
                code.push_str(&format!("class {}:\n", struct_def.name));
                if !struct_def.doc.is_empty() {
                    code.push_str(&format!("    \"\"\"{}\"\"\"\n", struct_def.doc));
                }
                code.push_str("    def __init__(self");
                for field in &struct_def.fields {
                    code.push_str(&format!(", {}", field.name));
                }
                code.push_str("):\n");
                for field in &struct_def.fields {
                    code.push_str(&format!("        self.{} = {}\n", field.name, field.name));
                }
            }
            Language::TypeScript => {
                if !struct_def.doc.is_empty() {
                    code.push_str(&format!("/** {} */\n", struct_def.doc));
                }
                code.push_str(&format!("export interface {} {{\n", struct_def.name));
                for field in &struct_def.fields {
                    if !field.doc.is_empty() {
                        code.push_str(&format!("  /** {} */\n", field.doc));
                    }
                    let field_type = self.map_type(&field.field_type)?;
                    code.push_str(&format!("  {}: {};\n", field.name, field_type));
                }
                code.push_str("}\n");
            }
            _ => {
                return Err(CodeGenError::UnsupportedType(format!(
                    "Struct generation not implemented for {}",
                    self.language
                )));
            }
        }

        Ok(code)
    }

    /// Generate enum code
    fn generate_enum(&self, enum_def: &EnumDef) -> Result<String, CodeGenError> {
        let mut code = String::new();

        match self.language {
            Language::Rust => {
                if !enum_def.doc.is_empty() {
                    code.push_str(&format!("/// {}\n", enum_def.doc));
                }
                code.push_str(&format!("pub enum {} {{\n", enum_def.name));
                for variant in &enum_def.variants {
                    if !variant.doc.is_empty() {
                        code.push_str(&format!("    /// {}\n", variant.doc));
                    }
                    code.push_str(&format!("    {},\n", variant.name));
                }
                code.push_str("}\n");
            }
            Language::TypeScript => {
                if !enum_def.doc.is_empty() {
                    code.push_str(&format!("/** {} */\n", enum_def.doc));
                }
                code.push_str(&format!("export enum {} {{\n", enum_def.name));
                for (i, variant) in enum_def.variants.iter().enumerate() {
                    if !variant.doc.is_empty() {
                        code.push_str(&format!("  /** {} */\n", variant.doc));
                    }
                    let value = variant.value.unwrap_or(i as i64);
                    code.push_str(&format!("  {} = {},\n", variant.name, value));
                }
                code.push_str("}\n");
            }
            _ => {
                return Err(CodeGenError::UnsupportedType(format!(
                    "Enum generation not implemented for {}",
                    self.language
                )));
            }
        }

        Ok(code)
    }

    /// Generate class code
    fn generate_class(&self, class_def: &ClassDef) -> Result<String, CodeGenError> {
        let mut code = String::new();

        match self.language {
            Language::Python => {
                let base = class_def.base.as_ref().map(|b| format!("({})", b)).unwrap_or_default();
                code.push_str(&format!("class {}{}:\n", class_def.name, base));
                if !class_def.doc.is_empty() {
                    code.push_str(&format!("    \"\"\"{}\"\"\"\n", class_def.doc));
                }
                
                // Constructor
                if !class_def.constructors.is_empty() {
                    code.push_str("    def __init__(self):\n");
                    code.push_str("        pass\n\n");
                }

                // Methods
                for method in &class_def.methods {
                    code.push_str(&self.generate_method(method)?);
                }
            }
            Language::TypeScript => {
                if !class_def.doc.is_empty() {
                    code.push_str(&format!("/** {} */\n", class_def.doc));
                }
                let base = class_def.base.as_ref().map(|b| format!(" extends {}", b)).unwrap_or_default();
                code.push_str(&format!("export class {}{} {{\n", class_def.name, base));
                
                // Fields
                for field in &class_def.fields {
                    let field_type = self.map_type(&field.field_type)?;
                    code.push_str(&format!("  {}: {};\n", field.name, field_type));
                }
                
                // Methods
                for method in &class_def.methods {
                    code.push_str(&self.generate_method(method)?);
                }
                
                code.push_str("}\n");
            }
            _ => {
                return Err(CodeGenError::UnsupportedType(format!(
                    "Class generation not implemented for {}",
                    self.language
                )));
            }
        }

        Ok(code)
    }

    /// Generate function code
    fn generate_function(&self, func: &FunctionDef) -> Result<String, CodeGenError> {
        let mut code = String::new();

        match self.language {
            Language::Rust => {
                if !func.doc.is_empty() {
                    code.push_str(&format!("/// {}\n", func.doc));
                }
                code.push_str("pub fn ");
                code.push_str(&func.name);
                code.push('(');
                for (i, param) in func.params.iter().enumerate() {
                    if i > 0 {
                        code.push_str(", ");
                    }
                    let param_type = self.map_type(&param.param_type)?;
                    code.push_str(&format!("{}: {}", param.name, param_type));
                }
                code.push(')');
                let return_type = self.map_type(&func.return_type)?;
                if return_type != "()" {
                    code.push_str(&format!(" -> {}", return_type));
                }
                code.push_str(" {\n    todo!()\n}\n");
            }
            Language::Python => {
                code.push_str(&format!("def {}(", func.name));
                for (i, param) in func.params.iter().enumerate() {
                    if i > 0 {
                        code.push_str(", ");
                    }
                    code.push_str(&param.name);
                }
                code.push_str("):\n");
                if !func.doc.is_empty() {
                    code.push_str(&format!("    \"\"\"{}\"\"\"\n", func.doc));
                }
                code.push_str("    pass\n");
            }
            Language::TypeScript => {
                if !func.doc.is_empty() {
                    code.push_str(&format!("/** {} */\n", func.doc));
                }
                code.push_str(&format!("export function {}(", func.name));
                for (i, param) in func.params.iter().enumerate() {
                    if i > 0 {
                        code.push_str(", ");
                    }
                    let param_type = self.map_type(&param.param_type)?;
                    code.push_str(&format!("{}: {}", param.name, param_type));
                }
                code.push(')');
                let return_type = self.map_type(&func.return_type)?;
                if return_type != "void" {
                    code.push_str(&format!(": {}", return_type));
                }
                code.push_str(" {\n  throw new Error('Not implemented');\n}\n");
            }
            _ => {
                return Err(CodeGenError::UnsupportedType(format!(
                    "Function generation not implemented for {}",
                    self.language
                )));
            }
        }

        Ok(code)
    }

    /// Generate method code
    fn generate_method(&self, method: &FunctionDef) -> Result<String, CodeGenError> {
        let mut code = String::new();

        match self.language {
            Language::Python => {
                code.push_str(&format!("    def {}(self", method.name));
                for param in &method.params {
                    code.push_str(&format!(", {}", param.name));
                }
                code.push_str("):\n");
                if !method.doc.is_empty() {
                    code.push_str(&format!("        \"\"\"{}\"\"\"\n", method.doc));
                }
                code.push_str("        pass\n\n");
            }
            Language::TypeScript => {
                if !method.doc.is_empty() {
                    code.push_str(&format!("  /** {} */\n", method.doc));
                }
                code.push_str(&format!("  {}(", method.name));
                for (i, param) in method.params.iter().enumerate() {
                    if i > 0 {
                        code.push_str(", ");
                    }
                    let param_type = self.map_type(&param.param_type)?;
                    code.push_str(&format!("{}: {}", param.name, param_type));
                }
                code.push(')');
                let return_type = self.map_type(&method.return_type)?;
                if return_type != "void" {
                    code.push_str(&format!(": {}", return_type));
                }
                code.push_str(" {\n    throw new Error('Not implemented');\n  }\n\n");
            }
            _ => {
                return Err(CodeGenError::UnsupportedType(format!(
                    "Method generation not implemented for {}",
                    self.language
                )));
            }
        }

        Ok(code)
    }

    /// Map IDL type to target language type
    fn map_type(&self, type_kind: &TypeKind) -> Result<String, CodeGenError> {
        match type_kind {
            TypeKind::Primitive(prim) => {
                let key = match prim {
                    PrimitiveType::Bool => "bool",
                    PrimitiveType::Int32 => "int32",
                    PrimitiveType::Int64 => "int64",
                    PrimitiveType::Float32 => "float32",
                    PrimitiveType::Float64 => "float64",
                    PrimitiveType::String => "string",
                    _ => return Err(CodeGenError::UnsupportedType(format!("{:?}", prim))),
                };
                Ok(self.type_map.get(key).cloned().unwrap_or_else(|| key.to_string()))
            }
            TypeKind::Array(inner) => {
                let inner_type = self.map_type(inner)?;
                match self.language {
                    Language::Rust => Ok(format!("Vec<{}>", inner_type)),
                    Language::Python => Ok("list".to_string()),
                    Language::TypeScript => Ok(format!("{}[]", inner_type)),
                    Language::CSharp => Ok(format!("List<{}>", inner_type)),
                    Language::Cpp => Ok(format!("std::vector<{}>", inner_type)),
                    Language::Go => Ok(format!("[]{}", inner_type)),
                    Language::Java => Ok(format!("ArrayList<{}>", inner_type)),
                    _ => Ok("array".to_string()),
                }
            }
            TypeKind::Optional(inner) => {
                let inner_type = self.map_type(inner)?;
                match self.language {
                    Language::Rust => Ok(format!("Option<{}>", inner_type)),
                    Language::TypeScript => Ok(format!("{} | null", inner_type)),
                    Language::CSharp => Ok(format!("{}?", inner_type)),
                    Language::Cpp => Ok(format!("std::optional<{}>", inner_type)),
                    Language::Swift => Ok(format!("{}?", inner_type)),
                    _ => Ok(inner_type),
                }
            }
            TypeKind::Void => {
                match self.language {
                    Language::Rust => Ok("()".to_string()),
                    Language::Python => Ok("None".to_string()),
                    Language::TypeScript | Language::JavaScript => Ok("void".to_string()),
                    Language::CSharp | Language::Java => Ok("void".to_string()),
                    Language::Cpp => Ok("void".to_string()),
                    Language::Go => Ok("".to_string()),
                    Language::Swift => Ok("Void".to_string()),
                    _ => Ok("void".to_string()),
                }
            }
            TypeKind::Custom(name) => Ok(name.clone()),
            _ => Err(CodeGenError::UnsupportedType(format!("{:?}", type_kind))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_extension() {
        assert_eq!(Language::Rust.extension(), "rs");
        assert_eq!(Language::Python.extension(), "py");
        assert_eq!(Language::TypeScript.extension(), "ts");
    }

    #[test]
    fn test_language_package_manager() {
        assert_eq!(Language::Rust.package_manager(), "cargo");
        assert_eq!(Language::Python.package_manager(), "pip");
        assert_eq!(Language::JavaScript.package_manager(), "npm");
    }

    #[test]
    fn test_code_generator_creation() {
        let generator = CodeGenerator::new(Language::Rust);
        assert_eq!(generator.language, Language::Rust);
    }

    #[test]
    fn test_type_mapping_rust() {
        let generator = CodeGenerator::new(Language::Rust);
        let result = generator.map_type(&TypeKind::Primitive(PrimitiveType::Int32)).unwrap();
        assert_eq!(result, "i32");
    }

    #[test]
    fn test_type_mapping_python() {
        let generator = CodeGenerator::new(Language::Python);
        let result = generator.map_type(&TypeKind::Primitive(PrimitiveType::Int32)).unwrap();
        assert_eq!(result, "int");
    }

    #[test]
    fn test_type_mapping_typescript() {
        let generator = CodeGenerator::new(Language::TypeScript);
        let result = generator.map_type(&TypeKind::Primitive(PrimitiveType::Int32)).unwrap();
        assert_eq!(result, "number");
    }

    #[test]
    fn test_array_type_mapping() {
        let generator = CodeGenerator::new(Language::Rust);
        let array_type = TypeKind::Array(Box::new(TypeKind::Primitive(PrimitiveType::Int32)));
        let result = generator.map_type(&array_type).unwrap();
        assert_eq!(result, "Vec<i32>");
    }

    #[test]
    fn test_optional_type_mapping() {
        let generator = CodeGenerator::new(Language::Rust);
        let optional_type = TypeKind::Optional(Box::new(TypeKind::Primitive(PrimitiveType::String)));
        let result = generator.map_type(&optional_type).unwrap();
        assert_eq!(result, "Option<String>");
    }

    #[test]
    fn test_void_type_mapping() {
        let generator = CodeGenerator::new(Language::Rust);
        let result = generator.map_type(&TypeKind::Void).unwrap();
        assert_eq!(result, "()");
    }

    #[test]
    fn test_generate_simple_api() {
        let mut api = ApiDefinition::new("test_api", "1.0.0");
        let func = FunctionDef::new("test_func");
        api.add_function(func);

        let generator = CodeGenerator::new(Language::Rust);
        let result = generator.generate(&api);
        assert!(result.is_ok());
    }

    #[test]
    fn test_generate_struct_rust() {
        let struct_def = StructDef::new("Point")
            .with_field(FieldDef::new("x", TypeKind::Primitive(PrimitiveType::Float32)))
            .with_field(FieldDef::new("y", TypeKind::Primitive(PrimitiveType::Float32)));

        let generator = CodeGenerator::new(Language::Rust);
        let code = generator.generate_struct(&struct_def).unwrap();
        assert!(code.contains("pub struct Point"));
        assert!(code.contains("pub x: f32"));
    }

    #[test]
    fn test_generate_enum_rust() {
        let enum_def = EnumDef::new("Color")
            .with_variant(EnumVariantDef::new("Red"))
            .with_variant(EnumVariantDef::new("Green"))
            .with_variant(EnumVariantDef::new("Blue"));

        let generator = CodeGenerator::new(Language::Rust);
        let code = generator.generate_enum(&enum_def).unwrap();
        assert!(code.contains("pub enum Color"));
        assert!(code.contains("Red"));
    }

    #[test]
    fn test_generate_function_rust() {
        let func = FunctionDef::new("add")
            .with_param(ParamDef::new("a", TypeKind::Primitive(PrimitiveType::Int32)))
            .with_param(ParamDef::new("b", TypeKind::Primitive(PrimitiveType::Int32)))
            .with_return_type(TypeKind::Primitive(PrimitiveType::Int32));

        let generator = CodeGenerator::new(Language::Rust);
        let code = generator.generate_function(&func).unwrap();
        assert!(code.contains("pub fn add"));
        assert!(code.contains("a: i32"));
        assert!(code.contains("-> i32"));
    }

    #[test]
    fn test_package_metadata() {
        let metadata = PackageMetadata::new("test_package", "1.0.0");
        assert_eq!(metadata.name, "test_package");
        assert_eq!(metadata.version, "1.0.0");
    }

    #[test]
    fn test_language_display() {
        assert_eq!(format!("{}", Language::Rust), "Rust");
        assert_eq!(format!("{}", Language::Python), "Python");
        assert_eq!(format!("{}", Language::CSharp), "C#");
    }

    #[test]
    fn test_custom_type_mapping() {
        let generator = CodeGenerator::new(Language::Rust);
        let custom_type = TypeKind::Custom("MyType".to_string());
        let result = generator.map_type(&custom_type).unwrap();
        assert_eq!(result, "MyType");
    }

    #[test]
    fn test_generate_struct_python() {
        let struct_def = StructDef::new("Point")
            .with_field(FieldDef::new("x", TypeKind::Primitive(PrimitiveType::Float32)))
            .with_field(FieldDef::new("y", TypeKind::Primitive(PrimitiveType::Float32)));

        let generator = CodeGenerator::new(Language::Python);
        let code = generator.generate_struct(&struct_def).unwrap();
        assert!(code.contains("class Point"));
        assert!(code.contains("def __init__"));
    }

    #[test]
    fn test_generate_function_python() {
        let func = FunctionDef::new("greet")
            .with_param(ParamDef::new("name", TypeKind::Primitive(PrimitiveType::String)))
            .with_doc("Greet a person");

        let generator = CodeGenerator::new(Language::Python);
        let code = generator.generate_function(&func).unwrap();
        assert!(code.contains("def greet"));
        assert!(code.contains("name"));
        assert!(code.contains("Greet a person"));
    }

    #[test]
    fn test_generate_struct_typescript() {
        let struct_def = StructDef::new("Point")
            .with_field(FieldDef::new("x", TypeKind::Primitive(PrimitiveType::Float32)))
            .with_field(FieldDef::new("y", TypeKind::Primitive(PrimitiveType::Float32)));

        let generator = CodeGenerator::new(Language::TypeScript);
        let code = generator.generate_struct(&struct_def).unwrap();
        assert!(code.contains("export interface Point"));
        assert!(code.contains("x: number"));
    }

    #[test]
    fn test_generate_enum_typescript() {
        let enum_def = EnumDef::new("Status")
            .with_variant(EnumVariantDef::new("Active").with_value(0))
            .with_variant(EnumVariantDef::new("Inactive").with_value(1));

        let generator = CodeGenerator::new(Language::TypeScript);
        let code = generator.generate_enum(&enum_def).unwrap();
        assert!(code.contains("export enum Status"));
        assert!(code.contains("Active = 0"));
    }

    #[test]
    fn test_generated_code_files() {
        let mut api = ApiDefinition::new("test", "1.0.0");
        let struct_def = StructDef::new("Point");
        api.add_struct(struct_def);

        let generator = CodeGenerator::new(Language::Rust);
        let result = generator.generate(&api).unwrap();
        
        assert!(!result.files.is_empty());
        assert_eq!(result.metadata.name, "test");
    }
}


//! # SDK Interface Definition Language (IDL)
//!
//! Defines the API surface for multi-language SDK generation.
//!
//! ## Features
//! - Language-agnostic API definitions
//! - Type mapping for multiple languages
//! - Documentation generation
//! - Code generation metadata
//! - Version tracking
//!
//! ## Example
//! ```no_run
//! use windjammer_game_framework::sdk_idl::{ApiDefinition, TypeDef, FunctionDef};
//!
//! let mut api = ApiDefinition::new("windjammer", "1.0.0");
//! api.add_function(FunctionDef::new("create_window")
//!     .with_return_type("Window")
//!     .with_doc("Creates a new game window"));
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// API definition for SDK generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiDefinition {
    /// API name
    pub name: String,
    /// API version
    pub version: String,
    /// Type definitions
    pub types: Vec<TypeDef>,
    /// Function definitions
    pub functions: Vec<FunctionDef>,
    /// Constant definitions
    pub constants: Vec<ConstantDef>,
    /// Enum definitions
    pub enums: Vec<EnumDef>,
    /// Struct definitions
    pub structs: Vec<StructDef>,
    /// Class definitions (for OOP languages)
    pub classes: Vec<ClassDef>,
    /// Module organization
    pub modules: Vec<ModuleDef>,
}

/// Type definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeDef {
    /// Type name
    pub name: String,
    /// Type kind
    pub kind: TypeKind,
    /// Documentation
    pub doc: String,
    /// Nullable
    pub nullable: bool,
    /// Generic parameters
    pub generics: Vec<String>,
}

/// Type kind
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TypeKind {
    /// Primitive type (int, float, bool, string)
    Primitive(PrimitiveType),
    /// Array type
    Array(Box<TypeKind>),
    /// Map/Dictionary type
    Map(Box<TypeKind>, Box<TypeKind>),
    /// Custom type (struct, class, enum)
    Custom(String),
    /// Function pointer/callback
    Function {
        params: Vec<TypeKind>,
        return_type: Box<TypeKind>,
    },
    /// Optional type
    Optional(Box<TypeKind>),
    /// Void/Unit type
    Void,
}

/// Primitive type
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PrimitiveType {
    Bool,
    Int8,
    Int16,
    Int32,
    Int64,
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    Float32,
    Float64,
    String,
    Char,
}

/// Function definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDef {
    /// Function name
    pub name: String,
    /// Parameters
    pub params: Vec<ParamDef>,
    /// Return type
    pub return_type: TypeKind,
    /// Documentation
    pub doc: String,
    /// Is async
    pub is_async: bool,
    /// Is static (not a method)
    pub is_static: bool,
    /// Deprecated
    pub deprecated: Option<String>,
    /// Since version
    pub since: Option<String>,
}

/// Parameter definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamDef {
    /// Parameter name
    pub name: String,
    /// Parameter type
    pub param_type: TypeKind,
    /// Default value
    pub default: Option<String>,
    /// Documentation
    pub doc: String,
}

/// Constant definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstantDef {
    /// Constant name
    pub name: String,
    /// Constant type
    pub const_type: TypeKind,
    /// Constant value
    pub value: String,
    /// Documentation
    pub doc: String,
}

/// Enum definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumDef {
    /// Enum name
    pub name: String,
    /// Enum variants
    pub variants: Vec<EnumVariantDef>,
    /// Documentation
    pub doc: String,
}

/// Enum variant definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumVariantDef {
    /// Variant name
    pub name: String,
    /// Variant value (for C-style enums)
    pub value: Option<i64>,
    /// Associated data (for Rust-style enums)
    pub data: Vec<TypeKind>,
    /// Documentation
    pub doc: String,
}

/// Struct definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructDef {
    /// Struct name
    pub name: String,
    /// Fields
    pub fields: Vec<FieldDef>,
    /// Documentation
    pub doc: String,
    /// Generic parameters
    pub generics: Vec<String>,
}

/// Field definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDef {
    /// Field name
    pub name: String,
    /// Field type
    pub field_type: TypeKind,
    /// Documentation
    pub doc: String,
    /// Public visibility
    pub public: bool,
}

/// Class definition (for OOP languages)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassDef {
    /// Class name
    pub name: String,
    /// Base class
    pub base: Option<String>,
    /// Interfaces
    pub interfaces: Vec<String>,
    /// Fields
    pub fields: Vec<FieldDef>,
    /// Methods
    pub methods: Vec<FunctionDef>,
    /// Constructors
    pub constructors: Vec<FunctionDef>,
    /// Documentation
    pub doc: String,
    /// Generic parameters
    pub generics: Vec<String>,
}

/// Module definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleDef {
    /// Module name
    pub name: String,
    /// Submodules
    pub submodules: Vec<String>,
    /// Types in this module
    pub types: Vec<String>,
    /// Functions in this module
    pub functions: Vec<String>,
    /// Documentation
    pub doc: String,
}

impl ApiDefinition {
    /// Create a new API definition
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            types: Vec::new(),
            functions: Vec::new(),
            constants: Vec::new(),
            enums: Vec::new(),
            structs: Vec::new(),
            classes: Vec::new(),
            modules: Vec::new(),
        }
    }

    /// Add a type definition
    pub fn add_type(&mut self, type_def: TypeDef) {
        self.types.push(type_def);
    }

    /// Add a function definition
    pub fn add_function(&mut self, function: FunctionDef) {
        self.functions.push(function);
    }

    /// Add a constant definition
    pub fn add_constant(&mut self, constant: ConstantDef) {
        self.constants.push(constant);
    }

    /// Add an enum definition
    pub fn add_enum(&mut self, enum_def: EnumDef) {
        self.enums.push(enum_def);
    }

    /// Add a struct definition
    pub fn add_struct(&mut self, struct_def: StructDef) {
        self.structs.push(struct_def);
    }

    /// Add a class definition
    pub fn add_class(&mut self, class: ClassDef) {
        self.classes.push(class);
    }

    /// Add a module definition
    pub fn add_module(&mut self, module: ModuleDef) {
        self.modules.push(module);
    }

    /// Export to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Import from JSON
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Export to YAML
    #[cfg(feature = "yaml")]
    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }

    /// Import from YAML
    #[cfg(feature = "yaml")]
    pub fn from_yaml(yaml: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml)
    }
}

impl TypeDef {
    /// Create a new type definition
    pub fn new(name: impl Into<String>, kind: TypeKind) -> Self {
        Self {
            name: name.into(),
            kind,
            doc: String::new(),
            nullable: false,
            generics: Vec::new(),
        }
    }

    /// Set documentation
    pub fn with_doc(mut self, doc: impl Into<String>) -> Self {
        self.doc = doc.into();
        self
    }

    /// Set nullable
    pub fn with_nullable(mut self, nullable: bool) -> Self {
        self.nullable = nullable;
        self
    }

    /// Add generic parameter
    pub fn with_generic(mut self, generic: impl Into<String>) -> Self {
        self.generics.push(generic.into());
        self
    }
}

impl FunctionDef {
    /// Create a new function definition
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            params: Vec::new(),
            return_type: TypeKind::Void,
            doc: String::new(),
            is_async: false,
            is_static: true,
            deprecated: None,
            since: None,
        }
    }

    /// Add parameter
    pub fn with_param(mut self, param: ParamDef) -> Self {
        self.params.push(param);
        self
    }

    /// Set return type
    pub fn with_return_type(mut self, return_type: TypeKind) -> Self {
        self.return_type = return_type;
        self
    }

    /// Set documentation
    pub fn with_doc(mut self, doc: impl Into<String>) -> Self {
        self.doc = doc.into();
        self
    }

    /// Set async
    pub fn with_async(mut self, is_async: bool) -> Self {
        self.is_async = is_async;
        self
    }

    /// Set static
    pub fn with_static(mut self, is_static: bool) -> Self {
        self.is_static = is_static;
        self
    }

    /// Set deprecated
    pub fn with_deprecated(mut self, message: impl Into<String>) -> Self {
        self.deprecated = Some(message.into());
        self
    }

    /// Set since version
    pub fn with_since(mut self, version: impl Into<String>) -> Self {
        self.since = Some(version.into());
        self
    }
}

impl ParamDef {
    /// Create a new parameter definition
    pub fn new(name: impl Into<String>, param_type: TypeKind) -> Self {
        Self {
            name: name.into(),
            param_type,
            default: None,
            doc: String::new(),
        }
    }

    /// Set default value
    pub fn with_default(mut self, default: impl Into<String>) -> Self {
        self.default = Some(default.into());
        self
    }

    /// Set documentation
    pub fn with_doc(mut self, doc: impl Into<String>) -> Self {
        self.doc = doc.into();
        self
    }
}

impl ConstantDef {
    /// Create a new constant definition
    pub fn new(name: impl Into<String>, const_type: TypeKind, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            const_type,
            value: value.into(),
            doc: String::new(),
        }
    }

    /// Set documentation
    pub fn with_doc(mut self, doc: impl Into<String>) -> Self {
        self.doc = doc.into();
        self
    }
}

impl EnumDef {
    /// Create a new enum definition
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            variants: Vec::new(),
            doc: String::new(),
        }
    }

    /// Add variant
    pub fn with_variant(mut self, variant: EnumVariantDef) -> Self {
        self.variants.push(variant);
        self
    }

    /// Set documentation
    pub fn with_doc(mut self, doc: impl Into<String>) -> Self {
        self.doc = doc.into();
        self
    }
}

impl EnumVariantDef {
    /// Create a new enum variant
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: None,
            data: Vec::new(),
            doc: String::new(),
        }
    }

    /// Set value
    pub fn with_value(mut self, value: i64) -> Self {
        self.value = Some(value);
        self
    }

    /// Add data
    pub fn with_data(mut self, data: TypeKind) -> Self {
        self.data.push(data);
        self
    }

    /// Set documentation
    pub fn with_doc(mut self, doc: impl Into<String>) -> Self {
        self.doc = doc.into();
        self
    }
}

impl StructDef {
    /// Create a new struct definition
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            fields: Vec::new(),
            doc: String::new(),
            generics: Vec::new(),
        }
    }

    /// Add field
    pub fn with_field(mut self, field: FieldDef) -> Self {
        self.fields.push(field);
        self
    }

    /// Set documentation
    pub fn with_doc(mut self, doc: impl Into<String>) -> Self {
        self.doc = doc.into();
        self
    }

    /// Add generic parameter
    pub fn with_generic(mut self, generic: impl Into<String>) -> Self {
        self.generics.push(generic.into());
        self
    }
}

impl FieldDef {
    /// Create a new field definition
    pub fn new(name: impl Into<String>, field_type: TypeKind) -> Self {
        Self {
            name: name.into(),
            field_type,
            doc: String::new(),
            public: true,
        }
    }

    /// Set documentation
    pub fn with_doc(mut self, doc: impl Into<String>) -> Self {
        self.doc = doc.into();
        self
    }

    /// Set visibility
    pub fn with_public(mut self, public: bool) -> Self {
        self.public = public;
        self
    }
}

impl ClassDef {
    /// Create a new class definition
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            base: None,
            interfaces: Vec::new(),
            fields: Vec::new(),
            methods: Vec::new(),
            constructors: Vec::new(),
            doc: String::new(),
            generics: Vec::new(),
        }
    }

    /// Set base class
    pub fn with_base(mut self, base: impl Into<String>) -> Self {
        self.base = Some(base.into());
        self
    }

    /// Add interface
    pub fn with_interface(mut self, interface: impl Into<String>) -> Self {
        self.interfaces.push(interface.into());
        self
    }

    /// Add field
    pub fn with_field(mut self, field: FieldDef) -> Self {
        self.fields.push(field);
        self
    }

    /// Add method
    pub fn with_method(mut self, method: FunctionDef) -> Self {
        self.methods.push(method);
        self
    }

    /// Add constructor
    pub fn with_constructor(mut self, constructor: FunctionDef) -> Self {
        self.constructors.push(constructor);
        self
    }

    /// Set documentation
    pub fn with_doc(mut self, doc: impl Into<String>) -> Self {
        self.doc = doc.into();
        self
    }

    /// Add generic parameter
    pub fn with_generic(mut self, generic: impl Into<String>) -> Self {
        self.generics.push(generic.into());
        self
    }
}

impl ModuleDef {
    /// Create a new module definition
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            submodules: Vec::new(),
            types: Vec::new(),
            functions: Vec::new(),
            doc: String::new(),
        }
    }

    /// Add submodule
    pub fn with_submodule(mut self, submodule: impl Into<String>) -> Self {
        self.submodules.push(submodule.into());
        self
    }

    /// Add type
    pub fn with_type(mut self, type_name: impl Into<String>) -> Self {
        self.types.push(type_name.into());
        self
    }

    /// Add function
    pub fn with_function(mut self, function: impl Into<String>) -> Self {
        self.functions.push(function.into());
        self
    }

    /// Set documentation
    pub fn with_doc(mut self, doc: impl Into<String>) -> Self {
        self.doc = doc.into();
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_definition_creation() {
        let api = ApiDefinition::new("test_api", "1.0.0");
        assert_eq!(api.name, "test_api");
        assert_eq!(api.version, "1.0.0");
    }

    #[test]
    fn test_add_function() {
        let mut api = ApiDefinition::new("test_api", "1.0.0");
        let func = FunctionDef::new("test_func")
            .with_doc("Test function");
        api.add_function(func);
        assert_eq!(api.functions.len(), 1);
        assert_eq!(api.functions[0].name, "test_func");
    }

    #[test]
    fn test_function_with_params() {
        let func = FunctionDef::new("add")
            .with_param(ParamDef::new("a", TypeKind::Primitive(PrimitiveType::Int32)))
            .with_param(ParamDef::new("b", TypeKind::Primitive(PrimitiveType::Int32)))
            .with_return_type(TypeKind::Primitive(PrimitiveType::Int32));

        assert_eq!(func.params.len(), 2);
        assert_eq!(func.params[0].name, "a");
    }

    #[test]
    fn test_struct_definition() {
        let struct_def = StructDef::new("Point")
            .with_field(FieldDef::new("x", TypeKind::Primitive(PrimitiveType::Float32)))
            .with_field(FieldDef::new("y", TypeKind::Primitive(PrimitiveType::Float32)))
            .with_doc("A 2D point");

        assert_eq!(struct_def.fields.len(), 2);
        assert_eq!(struct_def.doc, "A 2D point");
    }

    #[test]
    fn test_enum_definition() {
        let enum_def = EnumDef::new("Color")
            .with_variant(EnumVariantDef::new("Red").with_value(0))
            .with_variant(EnumVariantDef::new("Green").with_value(1))
            .with_variant(EnumVariantDef::new("Blue").with_value(2));

        assert_eq!(enum_def.variants.len(), 3);
        assert_eq!(enum_def.variants[0].value, Some(0));
    }

    #[test]
    fn test_class_definition() {
        let class = ClassDef::new("GameObject")
            .with_field(FieldDef::new("name", TypeKind::Primitive(PrimitiveType::String)))
            .with_method(FunctionDef::new("update").with_static(false))
            .with_doc("Base game object class");

        assert_eq!(class.fields.len(), 1);
        assert_eq!(class.methods.len(), 1);
    }

    #[test]
    fn test_type_def() {
        let type_def = TypeDef::new("Vector3", TypeKind::Custom("Vec3".to_string()))
            .with_doc("3D vector")
            .with_nullable(false);

        assert_eq!(type_def.name, "Vector3");
        assert!(!type_def.nullable);
    }

    #[test]
    fn test_constant_def() {
        let const_def = ConstantDef::new(
            "PI",
            TypeKind::Primitive(PrimitiveType::Float64),
            "3.14159265359"
        ).with_doc("Mathematical constant PI");

        assert_eq!(const_def.name, "PI");
        assert_eq!(const_def.value, "3.14159265359");
    }

    #[test]
    fn test_module_def() {
        let module = ModuleDef::new("math")
            .with_function("add")
            .with_function("subtract")
            .with_type("Vector3")
            .with_doc("Math utilities");

        assert_eq!(module.functions.len(), 2);
        assert_eq!(module.types.len(), 1);
    }

    #[test]
    fn test_json_serialization() {
        let api = ApiDefinition::new("test_api", "1.0.0");
        let json = api.to_json().unwrap();
        assert!(json.contains("test_api"));
        assert!(json.contains("1.0.0"));
    }

    #[test]
    fn test_json_deserialization() {
        let api = ApiDefinition::new("test_api", "1.0.0");
        let json = api.to_json().unwrap();
        let deserialized = ApiDefinition::from_json(&json).unwrap();
        assert_eq!(deserialized.name, "test_api");
        assert_eq!(deserialized.version, "1.0.0");
    }

    #[test]
    fn test_param_with_default() {
        let param = ParamDef::new("timeout", TypeKind::Primitive(PrimitiveType::Int32))
            .with_default("1000")
            .with_doc("Timeout in milliseconds");

        assert_eq!(param.default, Some("1000".to_string()));
    }

    #[test]
    fn test_function_deprecated() {
        let func = FunctionDef::new("old_func")
            .with_deprecated("Use new_func instead")
            .with_since("1.0.0");

        assert!(func.deprecated.is_some());
        assert_eq!(func.since, Some("1.0.0".to_string()));
    }

    #[test]
    fn test_generic_struct() {
        let struct_def = StructDef::new("Container")
            .with_generic("T")
            .with_field(FieldDef::new("value", TypeKind::Custom("T".to_string())));

        assert_eq!(struct_def.generics.len(), 1);
        assert_eq!(struct_def.generics[0], "T");
    }

    #[test]
    fn test_array_type() {
        let array_type = TypeKind::Array(Box::new(TypeKind::Primitive(PrimitiveType::Int32)));
        let type_def = TypeDef::new("IntArray", array_type);
        
        match type_def.kind {
            TypeKind::Array(_) => (),
            _ => panic!("Expected array type"),
        }
    }

    #[test]
    fn test_map_type() {
        let map_type = TypeKind::Map(
            Box::new(TypeKind::Primitive(PrimitiveType::String)),
            Box::new(TypeKind::Primitive(PrimitiveType::Int32))
        );
        let type_def = TypeDef::new("StringIntMap", map_type);
        
        match type_def.kind {
            TypeKind::Map(_, _) => (),
            _ => panic!("Expected map type"),
        }
    }

    #[test]
    fn test_optional_type() {
        let optional_type = TypeKind::Optional(Box::new(TypeKind::Primitive(PrimitiveType::String)));
        let type_def = TypeDef::new("OptionalString", optional_type);
        
        match type_def.kind {
            TypeKind::Optional(_) => (),
            _ => panic!("Expected optional type"),
        }
    }

    #[test]
    fn test_function_type() {
        let func_type = TypeKind::Function {
            params: vec![TypeKind::Primitive(PrimitiveType::Int32)],
            return_type: Box::new(TypeKind::Primitive(PrimitiveType::Bool)),
        };
        let type_def = TypeDef::new("Predicate", func_type);
        
        match type_def.kind {
            TypeKind::Function { .. } => (),
            _ => panic!("Expected function type"),
        }
    }

    #[test]
    fn test_class_inheritance() {
        let class = ClassDef::new("Player")
            .with_base("GameObject")
            .with_interface("IMovable")
            .with_interface("IDamageable");

        assert_eq!(class.base, Some("GameObject".to_string()));
        assert_eq!(class.interfaces.len(), 2);
    }

    #[test]
    fn test_enum_with_data() {
        let variant = EnumVariantDef::new("Point")
            .with_data(TypeKind::Primitive(PrimitiveType::Float32))
            .with_data(TypeKind::Primitive(PrimitiveType::Float32));

        assert_eq!(variant.data.len(), 2);
    }

    #[test]
    fn test_async_function() {
        let func = FunctionDef::new("fetch_data")
            .with_async(true)
            .with_return_type(TypeKind::Primitive(PrimitiveType::String));

        assert!(func.is_async);
    }

    #[test]
    fn test_field_visibility() {
        let field = FieldDef::new("private_data", TypeKind::Primitive(PrimitiveType::Int32))
            .with_public(false);

        assert!(!field.public);
    }
}


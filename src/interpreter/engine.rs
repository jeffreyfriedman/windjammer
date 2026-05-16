//! The Windjammerscript tree-walking interpreter engine.
//!
//! Walks the AST directly, evaluating expressions and executing statements.
//! Uses the same parser as the compiler backends — same `.wj` source runs
//! everywhere.

use super::environment::Environment;
use super::value::Value;
use crate::parser::{FunctionDecl, Item, Program};
use std::collections::HashMap;

/// Control flow signal returned by statement execution
pub(crate) enum ControlFlow {
    Continue,
    Return(Value),
    Break,
    LoopContinue,
}

/// Stored function definition
pub(crate) struct FunctionDef<'a> {
    pub(crate) decl: &'a FunctionDecl<'a>,
    #[allow(dead_code)]
    pub(crate) receiver_type: Option<String>,
}

/// Information about an enum variant for runtime construction
#[derive(Debug, Clone)]
pub(crate) struct EnumVariantInfo {
    pub(crate) enum_name: String,
    pub(crate) variant_name: String,
    pub(crate) data_kind: EnumVariantKind,
}

#[derive(Debug, Clone)]
pub(crate) enum EnumVariantKind {
    Unit,
    Tuple {
        #[allow(dead_code)]
        count: usize,
    },
    Struct {
        #[allow(dead_code)]
        field_names: Vec<String>,
    },
}

/// The Windjammerscript interpreter
#[derive(Default)]
pub struct Interpreter<'a> {
    pub(crate) env: Environment,
    /// All function/method definitions indexed by name
    pub(crate) functions: HashMap<String, Vec<FunctionDef<'a>>>,
    /// Struct definitions: name → field names
    pub(crate) struct_defs: HashMap<String, Vec<String>>,
    /// Enum variant info: "EnumName::Variant" → EnumVariantInfo
    pub(crate) enum_variants: HashMap<String, EnumVariantInfo>,
    /// Captured stdout (for testing)
    pub(crate) output: Vec<String>,
    /// Whether to capture output instead of printing
    pub(crate) capture_output: bool,
}

impl<'a> Interpreter<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create an interpreter that captures output (for testing)
    pub fn new_capturing() -> Self {
        Self {
            env: Environment::new(),
            functions: HashMap::new(),
            struct_defs: HashMap::new(),
            enum_variants: HashMap::new(),
            output: Vec::new(),
            capture_output: true,
        }
    }

    /// Get captured output
    pub fn get_output(&self) -> String {
        self.output.join("")
    }

    /// Run a complete program
    pub fn run(&mut self, program: &'a Program<'a>) -> Result<Value, String> {
        self.register_definitions(program);

        if self.functions.contains_key("main") {
            self.call_function("main", &[])
        } else {
            Err("No main() function found".to_string())
        }
    }

    /// Register all top-level definitions
    fn register_definitions(&mut self, program: &'a Program<'a>) {
        for item in &program.items {
            match item {
                Item::Function { decl, .. } => {
                    let name = decl.name.clone();
                    self.functions.entry(name).or_default().push(FunctionDef {
                        decl,
                        receiver_type: None,
                    });
                }
                Item::Struct { decl, .. } => {
                    let field_names: Vec<String> =
                        decl.fields.iter().map(|f| f.name.clone()).collect();
                    self.struct_defs.insert(decl.name.clone(), field_names);
                }
                Item::Impl { block, .. } => {
                    let type_name = block.type_name.clone();
                    for method in &block.functions {
                        let method_key = format!("{}::{}", type_name, method.name);
                        self.functions
                            .entry(method_key)
                            .or_default()
                            .push(FunctionDef {
                                decl: method,
                                receiver_type: Some(type_name.clone()),
                            });
                    }
                }
                Item::Enum { decl, .. } => {
                    for variant in &decl.variants {
                        let key = format!("{}::{}", decl.name, variant.name);
                        let data_kind = match &variant.data {
                            crate::parser::EnumVariantData::Unit => EnumVariantKind::Unit,
                            crate::parser::EnumVariantData::Tuple(types) => {
                                EnumVariantKind::Tuple { count: types.len() }
                            }
                            crate::parser::EnumVariantData::Struct(fields) => {
                                EnumVariantKind::Struct {
                                    field_names: fields
                                        .iter()
                                        .map(|(name, _)| name.clone())
                                        .collect(),
                                }
                            }
                        };
                        self.enum_variants.insert(
                            key,
                            EnumVariantInfo {
                                enum_name: decl.name.clone(),
                                variant_name: variant.name.clone(),
                                data_kind,
                            },
                        );
                    }
                }
                Item::Const { name, value, .. } => {
                    let val = self.eval_expression(value);
                    self.env.define(name, val);
                }
                _ => {}
            }
        }
    }

    /// Call a named function with arguments
    pub(crate) fn call_function(&mut self, name: &str, args: &[Value]) -> Result<Value, String> {
        let decl = {
            let func_defs = self
                .functions
                .get(name)
                .ok_or_else(|| format!("Undefined function: {}", name))?;
            let func_def = func_defs
                .first()
                .ok_or_else(|| format!("No definition for function: {}", name))?;
            func_def.decl
        };

        self.env.push_scope();

        let param_iter = decl.parameters.iter().filter(|p| p.name != "self");

        for (param, arg) in param_iter.zip(args.iter()) {
            self.env.define(&param.name, arg.clone());
        }

        let result = self.exec_body(&decl.body);

        self.env.pop_scope();

        match result {
            ControlFlow::Return(val) => Ok(val),
            ControlFlow::Continue => Ok(Value::Unit),
            _ => Ok(Value::Unit),
        }
    }
}

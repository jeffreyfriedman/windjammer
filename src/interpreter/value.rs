//! Runtime values for the Windjammerscript interpreter.
//!
//! Values use reference semantics — no ownership tracking.
//! The compiler handles safety; the interpreter handles execution.

use std::collections::HashMap;
use std::fmt;

/// Runtime value in the interpreter
#[derive(Debug, Clone)]
pub enum Value {
    /// Integer (i64)
    Int(i64),
    /// Floating point (f64)
    Float(f64),
    /// Boolean
    Bool(bool),
    /// String
    String(String),
    /// Character
    Char(char),
    /// A vector/array of values
    Vec(Vec<Value>),
    /// A struct instance: type name + field map
    Struct {
        type_name: String,
        fields: HashMap<String, Value>,
    },
    /// An enum variant
    Enum {
        type_name: String,
        variant: String,
        data: EnumData,
    },
    /// A closure / function reference
    Function(FunctionValue),
    /// Option::None / null
    Nil,
    /// Unit type (void return)
    Unit,
    /// A HashMap
    Map(HashMap<String, Value>),
    /// A tuple
    Tuple(Vec<Value>),
}

/// Data carried by an enum variant
#[derive(Debug, Clone)]
pub enum EnumData {
    Unit,
    Tuple(Vec<Value>),
    Struct(HashMap<String, Value>),
}

/// A function value (for closures and function references)
#[derive(Debug, Clone)]
pub struct FunctionValue {
    pub name: String,
    pub params: Vec<String>,
    /// Index into the interpreter's function table
    pub body_id: usize,
}

impl Value {
    /// Check if this value is truthy
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Int(n) => *n != 0,
            Value::Float(f) => *f != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::Nil => false,
            Value::Unit => false,
            _ => true,
        }
    }

    /// Get the type name of this value (for error messages)
    pub fn type_name(&self) -> &str {
        match self {
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::Bool(_) => "bool",
            Value::String(_) => "string",
            Value::Char(_) => "char",
            Value::Vec(_) => "Vec",
            Value::Struct { type_name, .. } => type_name,
            Value::Enum { type_name, .. } => type_name,
            Value::Function(_) => "function",
            Value::Nil => "nil",
            Value::Unit => "()",
            Value::Map(_) => "HashMap",
            Value::Tuple(_) => "tuple",
        }
    }

    /// Convert to integer, if possible
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(n) => Some(*n),
            Value::Float(f) => Some(*f as i64),
            Value::Bool(b) => Some(if *b { 1 } else { 0 }),
            _ => None,
        }
    }

    /// Convert to float, if possible
    pub fn as_float(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            Value::Int(n) => Some(*n as f64),
            _ => None,
        }
    }

    /// Convert to string representation
    pub fn to_display_string(&self) -> String {
        match self {
            Value::Int(n) => n.to_string(),
            Value::Float(f) => {
                let s = f.to_string();
                if s.contains('.') { s } else { format!("{}.0", s) }
            }
            Value::Bool(b) => b.to_string(),
            Value::String(s) => s.clone(),
            Value::Char(c) => c.to_string(),
            Value::Vec(items) => {
                let strs: Vec<String> = items.iter().map(|v| v.to_display_string()).collect();
                format!("[{}]", strs.join(", "))
            }
            Value::Struct { type_name, fields } => {
                let field_strs: Vec<String> = fields
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, v.to_display_string()))
                    .collect();
                format!("{} {{ {} }}", type_name, field_strs.join(", "))
            }
            Value::Enum { variant, data, .. } => match data {
                EnumData::Unit => variant.clone(),
                EnumData::Tuple(vals) => {
                    let strs: Vec<String> = vals.iter().map(|v| v.to_display_string()).collect();
                    format!("{}({})", variant, strs.join(", "))
                }
                EnumData::Struct(fields) => {
                    let strs: Vec<String> = fields
                        .iter()
                        .map(|(k, v)| format!("{}: {}", k, v.to_display_string()))
                        .collect();
                    format!("{} {{ {} }}", variant, strs.join(", "))
                }
            },
            Value::Function(f) => format!("<fn {}>", f.name),
            Value::Nil => "None".to_string(),
            Value::Unit => "()".to_string(),
            Value::Map(m) => {
                let strs: Vec<String> = m
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, v.to_display_string()))
                    .collect();
                format!("{{{}}}", strs.join(", "))
            }
            Value::Tuple(items) => {
                let strs: Vec<String> = items.iter().map(|v| v.to_display_string()).collect();
                format!("({})", strs.join(", "))
            }
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_display_string())
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Char(a), Value::Char(b)) => a == b,
            (Value::Nil, Value::Nil) => true,
            (Value::Unit, Value::Unit) => true,
            _ => false,
        }
    }
}

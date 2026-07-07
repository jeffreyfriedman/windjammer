impl IntInference {
    fn extract_int_type(&self, ty: &Type) -> Option<IntType> {
        self.extract_nested_int_type(ty)
    }

    /// Extract int type from nested generics: Option<Vec<int>>, Vec<Option<int>>, etc.
    fn extract_nested_int_type(&self, ty: &Type) -> Option<IntType> {
        match ty {
            Type::Int32 => Some(IntType::I32),
            Type::Int => Some(IntType::I64),
            Type::Uint => Some(IntType::U64),
            Type::Custom(name) => match name.as_str() {
                "i32" => Some(IntType::I32),
                "i64" => Some(IntType::I64),
                "u32" => Some(IntType::U32),
                "u64" => Some(IntType::U64),
                "usize" => Some(IntType::Usize),
                "isize" => Some(IntType::Isize),
                "u8" => Some(IntType::U8),
                "i8" => Some(IntType::I8),
                "u16" => Some(IntType::U16),
                "i16" => Some(IntType::I16),
                _ => None,
            },
            Type::Tuple(types) => {
                for t in types {
                    if let Some(it) = self.extract_nested_int_type(t) {
                        return Some(it);
                    }
                }
                None
            }
            Type::Vec(inner) => self.extract_nested_int_type(inner),
            Type::Array(inner, _) => self.extract_nested_int_type(inner),
            Type::Parameterized(name, args) if name == "Vec" && !args.is_empty() => {
                self.extract_nested_int_type(&args[0])
            }
            Type::Parameterized(name, args) if name == "Option" && !args.is_empty() => {
                self.extract_nested_int_type(&args[0])
            }
            Type::Parameterized(name, args) if name == "HashMap" && args.len() >= 2 => {
                self.extract_nested_int_type(&args[1])
            }
            Type::Parameterized(name, args) if name == "BTreeMap" && args.len() >= 2 => {
                self.extract_nested_int_type(&args[1])
            }
            Type::Reference(inner) | Type::MutableReference(inner) => {
                self.extract_nested_int_type(inner)
            }
            Type::Option(inner) => self.extract_nested_int_type(inner),
            Type::Result(ok, err) => self
                .extract_nested_int_type(ok)
                .or_else(|| self.extract_nested_int_type(err)),
            _ => None,
        }
    }

    /// Extract inner type T from Option<T> (handles Option, Parameterized, Reference)
    fn extract_option_inner_type(&self, ty: &Type) -> Option<Type> {
        match ty {
            Type::Option(inner) => Some((**inner).clone()),
            Type::Parameterized(name, args) if name == "Option" && !args.is_empty() => {
                Some(args[0].clone())
            }
            Type::Reference(inner) | Type::MutableReference(inner) => {
                self.extract_option_inner_type(inner)
            }
            _ => None,
        }
    }

    /// Infer Type from an expression (for receiver type resolution in method calls)
    /// TDD: Enables HashMap<K,V>.insert and Vec<T>.push generic type propagation
    fn infer_type_from_expression<'ast>(&self, expr: &Expression<'ast>) -> Option<Type> {
        match expr {
            Expression::StructLiteral { name, .. } => Some(Type::Custom(name.clone())),
            Expression::Call { function, .. } => {
                let func_name = match function {
                    Expression::Identifier { name, .. } => Some(name.clone()),
                    Expression::FieldAccess { object, field, .. } => {
                        if let Expression::Identifier {
                            name: type_name, ..
                        } = object
                        {
                            Some(format!("{}::{}", type_name, field))
                        } else {
                            None
                        }
                    }
                    _ => None,
                };
                func_name.and_then(|name| {
                    self.function_signatures
                        .get(&name)
                        .and_then(|(_, ret)| ret.clone())
                })
            }
            Expression::Identifier { name, .. } => {
                if name == "self" {
                    self.current_impl_type
                        .as_ref()
                        .map(|s| Type::Custom(s.clone()))
                } else {
                    self.var_types
                        .get(name)
                        .or_else(|| self.const_types.get(name))
                        .cloned()
                }
            }
            Expression::FieldAccess { object, field, .. } => {
                let object_type = self.infer_type_from_expression(object)?;
                let struct_name = match &object_type {
                    Type::Custom(name) => name.clone(),
                    _ => return None,
                };
                // TDD FIX: Strip generic params for struct field lookup
                // `ObjectPool<T>` → `ObjectPool`
                let base_name = if let Some(idx) = struct_name.find('<') {
                    &struct_name[..idx]
                } else {
                    &struct_name
                };
                let fields = if matches!(
                    *object,
                    Expression::Identifier { ref name, .. } if name == "self"
                ) {
                    self.current_impl_type
                        .as_deref()
                        .and_then(|ty| self.lookup_struct_fields_for_impl_type(ty))
                } else {
                    self.lookup_struct_fields(base_name)
                };
                fields.and_then(|m| m.get(field)).cloned()
            }
            Expression::Index { object, .. } => {
                let object_type = self.infer_type_from_expression(object)?;
                self.extract_vec_element_type(&object_type)
            }
            Expression::Cast { type_, .. } => Some(type_.clone()),
            Expression::Binary { left, op, .. } => {
                // TDD FIX: Binary operations return the type of their operands
                // For arithmetic (Add, Sub, Mul, Div, Mod): result type = operand type
                // For comparison (Eq, Lt, Gt, etc.): result type = bool
                use crate::parser::ast::operators::BinaryOp;
                match op {
                    BinaryOp::Add
                    | BinaryOp::Sub
                    | BinaryOp::Mul
                    | BinaryOp::Div
                    | BinaryOp::Mod
                    | BinaryOp::BitAnd
                    | BinaryOp::BitOr
                    | BinaryOp::BitXor
                    | BinaryOp::Shl
                    | BinaryOp::Shr => {
                        // Arithmetic: result has same type as operands
                        self.infer_type_from_expression(left)
                    }
                    BinaryOp::Eq
                    | BinaryOp::Ne
                    | BinaryOp::Lt
                    | BinaryOp::Le
                    | BinaryOp::Gt
                    | BinaryOp::Ge => {
                        // Comparison: result is bool
                        Some(Type::Bool)
                    }
                    BinaryOp::And | BinaryOp::Or => {
                        // Logical: result is bool
                        Some(Type::Bool)
                    }
                }
            }
            _ => None,
        }
    }

    /// Extract key type K from HashMap<K,V> or BTreeMap<K,V>
    fn extract_map_key_type(&self, ty: &Type) -> Option<Type> {
        match ty {
            Type::Parameterized(name, args) => {
                let base = crate::type_inference::generic_type_base_name(name);
                if matches!(base, "HashMap" | "BTreeMap" | "Map") && args.len() >= 2 {
                    Some(args[0].clone())
                } else {
                    None
                }
            }
            Type::Custom(name) if name.contains('<') => {
                let base = name.split('<').next().unwrap_or(name);
                if matches!(base, "HashMap" | "BTreeMap" | "Map") {
                    if let (Some(start), Some(end)) = (name.find('<'), name.rfind('>')) {
                        let inner = &name[start + 1..end];
                        let key = inner.split(',').next()?.trim();
                        return Some(self.parse_type_from_string(key));
                    }
                }
                None
            }
            Type::Reference(inner) | Type::MutableReference(inner) => {
                self.extract_map_key_type(inner)
            }
            _ => None,
        }
    }

    /// Extract value type V from HashMap<K,V> or BTreeMap<K,V>
    fn extract_map_value_type(&self, ty: &Type) -> Option<Type> {
        match ty {
            Type::Parameterized(name, args) => {
                let base = crate::type_inference::generic_type_base_name(name);
                if matches!(base, "HashMap" | "BTreeMap" | "Map") && args.len() >= 2 {
                    Some(args[1].clone())
                } else {
                    None
                }
            }
            Type::Custom(name) if name.contains('<') => {
                let base = name.split('<').next().unwrap_or(name);
                if matches!(base, "HashMap" | "BTreeMap" | "Map") {
                    if let (Some(start), Some(end)) = (name.find('<'), name.rfind('>')) {
                        let inner = &name[start + 1..end];
                        let value = inner.split(',').nth(1)?.trim();
                        return Some(self.parse_type_from_string(value));
                    }
                }
                None
            }
            Type::Reference(inner) | Type::MutableReference(inner) => {
                self.extract_map_value_type(inner)
            }
            _ => None,
        }
    }

    /// Extract element type T from Vec<T>
    fn extract_vec_element_type(&self, ty: &Type) -> Option<Type> {
        match ty {
            Type::Vec(inner) => Some((**inner).clone()),
            Type::Parameterized(name, args) if name == "Vec" && !args.is_empty() => {
                Some(args[0].clone())
            }
            Type::Reference(inner) | Type::MutableReference(inner) => {
                self.extract_vec_element_type(inner)
            }
            _ => None,
        }
    }
}

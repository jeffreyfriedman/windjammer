impl GoGenerator {
    fn enum_variant_to_go_type(&self, variant_name: &str) -> String {
        if let Some((type_name, variant)) = variant_name.split_once("::") {
            format!(
                "{}{}",
                Self::capitalize(type_name),
                Self::capitalize(variant)
            )
        } else {
            variant_name.to_string()
        }
    }

    fn type_to_go(&self, type_: &Type) -> String {
        match type_ {
            Type::Int | Type::Int32 => "int64".to_string(),
            Type::Uint => "uint64".to_string(),
            Type::Float => "float64".to_string(),
            Type::Bool => "bool".to_string(),
            Type::String => "string".to_string(),
            Type::Custom(name) => match name.as_str() {
                "int" => "int64".to_string(),
                "float" => "float64".to_string(),
                "bool" => "bool".to_string(),
                "string" => "string".to_string(),
                "usize" => "int".to_string(),
                "char" => "rune".to_string(),
                _ => name.clone(),
            },
            Type::Vec(inner) => format!("[]{}", self.type_to_go(inner)),
            Type::Array(inner, size) => format!("[{}]{}", size, self.type_to_go(inner)),
            Type::Option(inner) => format!("*{}", self.type_to_go(inner)),
            Type::Result(ok, _err) => self.type_to_go(ok),
            Type::Parameterized(name, args) => match name.as_str() {
                "Vec" => {
                    if let Some(inner) = args.first() {
                        format!("[]{}", self.type_to_go(inner))
                    } else {
                        "[]interface{}".to_string()
                    }
                }
                "HashMap" | "Map" => {
                    if args.len() >= 2 {
                        format!(
                            "map[{}]{}",
                            self.type_to_go(&args[0]),
                            self.type_to_go(&args[1])
                        )
                    } else {
                        "map[string]interface{}".to_string()
                    }
                }
                "Option" => {
                    if let Some(inner) = args.first() {
                        format!("*{}", self.type_to_go(inner))
                    } else {
                        "*interface{}".to_string()
                    }
                }
                _ => name.clone(),
            },
            Type::Generic(name) => format!("{} /* generic */", name),
            Type::Reference(inner) | Type::MutableReference(inner) => self.type_to_go(inner),
            Type::RawPointer { pointee, .. } => {
                format!("unsafe.Pointer /* *{} */", self.type_to_go(pointee))
            }
            Type::Tuple(types) => {
                if types.is_empty() {
                    String::new()
                } else {
                    format!("/* tuple */ {}", self.type_to_go(&types[0]))
                }
            }
            Type::TraitObject(name) => format!("{} /* interface */", name),
            Type::ImplTrait(name) => name.clone(),
            Type::Associated(base, assoc) => format!("/* {}.{} */ interface{{}}", base, assoc),
            Type::Infer => "interface{}".to_string(),
            Type::FunctionPointer {
                params,
                return_type,
            } => {
                let param_types: Vec<String> = params.iter().map(|t| self.type_to_go(t)).collect();
                let ret = match return_type {
                    Some(t) => format!(" {}", self.type_to_go(t)),
                    None => String::new(),
                };
                format!("func({}){}", param_types.join(", "), ret)
            }
        }
    }

    fn pattern_to_go(&self, pattern: &Pattern) -> String {
        match pattern {
            Pattern::Identifier(name) => name.clone(),
            Pattern::Wildcard => "_".to_string(),
            Pattern::Tuple(patterns) => {
                let parts: Vec<String> = patterns.iter().map(|p| self.pattern_to_go(p)).collect();
                parts.join(", ")
            }
            _ => "_".to_string(),
        }
    }
}

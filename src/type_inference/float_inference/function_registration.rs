impl FloatInference {
    fn register_const_and_static<'ast>(&mut self, item: &Item<'ast>) {
        match item {
            Item::Const { name, type_, .. } => {
                self.const_types.insert(name.clone(), type_.clone());
            }
            Item::Static { name, type_, .. } => {
                self.const_types.insert(name.clone(), type_.clone());
            }
            Item::Mod { items, .. } => {
                for sub_item in items {
                    self.register_const_and_static(sub_item);
                }
            }
            _ => {}
        }
    }

    /// Register function signatures for constraint propagation
    fn register_function_signature<'ast>(&mut self, item: &Item<'ast>) {
        match item {
            Item::Function { decl, .. } => {
                let param_types: Vec<Type> =
                    decl.parameters.iter().map(|p| p.type_.clone()).collect();

                self.function_signatures
                    .insert(decl.name.clone(), (param_types, decl.return_type.clone()));
            }
            Item::Impl { block, .. } => {
                // Register associated functions (e.g., Vec3::new)
                let type_name = block.type_name.clone();
                for func_decl in &block.functions {
                    let param_types: Vec<Type> = func_decl
                        .parameters
                        .iter()
                        .map(|p| p.type_.clone())
                        .collect();

                    // Register as "TypeName::method_name"
                    let full_name = format!("{}::{}", type_name, func_decl.name);
                    self.function_signatures
                        .insert(full_name, (param_types, func_decl.return_type.clone()));
                }
            }
            _ => {}
        }
    }
}

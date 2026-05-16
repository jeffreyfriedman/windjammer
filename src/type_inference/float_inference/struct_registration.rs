impl FloatInference {
    fn register_struct_fields_for_module<'ast>(
        &mut self,
        item: &Item<'ast>,
        module_prefix: &[String],
    ) {
        match item {
            Item::Struct { decl, .. } => {
                let key = struct_field_registry::qualify_struct_key(module_prefix, &decl.name);
                let mut field_map = HashMap::new();
                for field in &decl.fields {
                    field_map.insert(field.name.clone(), field.field_type.clone());
                }
                self.struct_field_types.insert(key, field_map);
            }
            Item::Enum { decl, .. } => {
                use crate::parser::EnumVariantData;
                for variant in &decl.variants {
                    if let EnumVariantData::Struct(fields) = &variant.data {
                        let variant_key = format!("{}::{}", decl.name, variant.name);
                        let qualified_key =
                            struct_field_registry::qualify_struct_key(module_prefix, &variant_key);
                        let mut field_map = HashMap::new();
                        for (name, ty) in fields {
                            field_map.insert(name.clone(), ty.clone());
                        }
                        self.struct_field_types
                            .insert(variant_key, field_map.clone());
                        self.struct_field_types.insert(qualified_key, field_map);
                    }
                }
            }
            Item::TypeAlias { name, target, .. } => {
                self.type_aliases.insert(name.clone(), target.clone());
            }
            Item::Mod { name, items, .. } => {
                let mut next = module_prefix.to_vec();
                next.push(name.clone());
                for sub_item in items {
                    self.register_struct_fields_for_module(sub_item, &next);
                }
            }
            _ => {}
        }
    }

    fn register_use_imports_from_items<'ast>(&mut self, items: &[Item<'ast>]) {
        for item in items {
            match item {
                Item::Use { path, alias, .. } => {
                    if path.len() == 1 && path[0].contains("::{") {
                        struct_field_registry::register_braced_use_imports(
                            &path[0],
                            &self.current_file_module_path,
                            &self.struct_field_types,
                            &self.struct_defining_module_paths,
                            &mut self.imported_type_registry_keys,
                        );
                        continue;
                    }
                    if path.last().map(|s| s.as_str()) == Some("*") {
                        struct_field_registry::expand_glob_import(
                            path,
                            &self.current_file_module_path,
                            &self.struct_field_types,
                            &self.struct_defining_module_paths,
                            &self.module_re_exports,
                            &mut self.imported_type_registry_keys,
                        );
                        continue;
                    }
                    if path.len() < 2 {
                        continue;
                    }
                    if let Some(key) = struct_field_registry::resolve_use_path_to_qualified_key(
                        path,
                        &self.current_file_module_path,
                        &self.struct_field_types,
                        &self.struct_defining_module_paths,
                    ) {
                        let imported_name = alias
                            .clone()
                            .unwrap_or_else(|| path.last().cloned().unwrap_or_default());
                        self.imported_type_registry_keys.insert(imported_name, key);
                    }
                }
                Item::Mod { items, .. } => self.register_use_imports_from_items(items),
                _ => {}
            }
        }
    }
}

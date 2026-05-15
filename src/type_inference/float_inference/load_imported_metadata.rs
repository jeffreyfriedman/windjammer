impl FloatInference {
    fn load_imported_metadata<'ast>(&mut self, program: &Program<'ast>) {
        use crate::metadata::{CrateMetadata, ModuleMetadata};
        use std::path::PathBuf;

        for item in &program.items {
            if let Item::Use { path, .. } = item {
                // Convert import path to file path
                // e.g., "crate::math::vec3::Vec3" → "math/vec3.wj.meta" (skip type name!)
                // e.g., "mylib::vec3::Vec3" → external crate, load from metadata.json
                let mut module_path: Vec<String> = path
                    .iter()
                    .skip_while(|s| {
                        s.as_str() == "crate" || s.as_str() == "self" || s.as_str() == "super"
                    })
                    .cloned()
                    .collect();

                if module_path.is_empty() {
                    continue;
                }

                // TDD FIX: Last element is type/function name, not module!
                let type_name = module_path.pop(); // Remove type name (Vec3)

                // CROSS-CRATE: Check for external crate metadata first
                if let (Some(crate_name), Some(ref ty_name)) = (module_path.first(), &type_name) {
                    let crate_key = crate_name.replace('-', "_");
                    if let Some(meta_dir) = self.external_crate_metadata_paths.get(&crate_key) {
                        let metadata_path = meta_dir.join("metadata.json");
                        if let Ok(meta_json) = std::fs::read_to_string(&metadata_path) {
                            if let Ok(crate_meta) =
                                serde_json::from_str::<CrateMetadata>(&meta_json)
                            {
                                // Load struct field types for the imported type
                                if let Some(fields) = crate_meta.structs.get(ty_name) {
                                    let mut field_map = HashMap::new();
                                    for (field_name, type_str) in fields {
                                        if let Some(field_type) =
                                            ModuleMetadata::deserialize_type(type_str)
                                        {
                                            field_map.insert(field_name.clone(), field_type);
                                        }
                                    }
                                    if !field_map.is_empty() {
                                        self.struct_field_types.insert(ty_name.clone(), field_map);
                                    }
                                }
                                // Load function signatures
                                for (func_name, sig) in &crate_meta.functions {
                                    let params: Vec<Type> = sig
                                        .params
                                        .iter()
                                        .filter_map(|s| ModuleMetadata::deserialize_type(s))
                                        .collect();
                                    let return_type = sig
                                        .return_type
                                        .as_ref()
                                        .and_then(|s| ModuleMetadata::deserialize_type(s));
                                    self.function_signatures
                                        .insert(func_name.clone(), (params, return_type));
                                }
                                continue; // Handled, skip .wj.meta lookup
                            }
                        }
                    }
                }

                if module_path.is_empty() {
                    continue; // Import from current module
                }

                // TDD FIX: Handle module re-exports by trying multiple paths
                // Example: "use crate::math::Vec3" could be:
                //   1. math/vec3.wj.meta (type defined in math/vec3.wj)
                //   2. math.wj.meta (type defined in math.wj)
                //   3. math/mod.wj.meta (type re-exported by math/mod.wj)

                // Build candidate paths
                let mut candidates: Vec<PathBuf> = Vec::new();

                if let Some(ref ty_name) = type_name {
                    // Helper: Convert PascalCase to snake_case
                    let snake_case = ty_name.chars().fold(String::new(), |mut acc, c| {
                        if c.is_uppercase() && !acc.is_empty() {
                            acc.push('_');
                        }
                        acc.push(c.to_lowercase().next().unwrap());
                        acc
                    });

                    // Helper: Truncate common suffixes (State, Config, Manager, etc.)
                    let truncated = ty_name
                        .to_lowercase()
                        .trim_end_matches("state")
                        .trim_end_matches("config")
                        .trim_end_matches("manager")
                        .trim_end_matches("system")
                        .to_string();

                    // Candidate 1: math/vec3.wj.meta (lowercase)
                    let mut p1 = PathBuf::new();
                    for seg in &module_path {
                        p1.push(seg);
                    }
                    p1.push(format!("{}.wj.meta", ty_name.to_lowercase()));
                    candidates.push(p1);

                    // Candidate 2: math/vec_3.wj.meta (snake_case)
                    let mut p2 = PathBuf::new();
                    for seg in &module_path {
                        p2.push(seg);
                    }
                    p2.push(format!("{}.wj.meta", snake_case));
                    candidates.push(p2);

                    // Candidate 3: ai/perception.wj.meta (truncated)
                    if !truncated.is_empty() && truncated != ty_name.to_lowercase() {
                        let mut p3 = PathBuf::new();
                        for seg in &module_path {
                            p3.push(seg);
                        }
                        p3.push(format!("{}.wj.meta", truncated));
                        candidates.push(p3);
                    }

                    // Candidate 4: math.wj.meta (mod file)
                    let mut p4 = PathBuf::new();
                    for (i, segment) in module_path.iter().enumerate() {
                        if i < module_path.len() - 1 {
                            p4.push(segment);
                        } else {
                            p4.push(format!("{}.wj.meta", segment));
                        }
                    }
                    candidates.push(p4);
                } else {
                    // No type name, just use module path
                    let mut meta_path = PathBuf::new();
                    for (i, segment) in module_path.iter().enumerate() {
                        if i < module_path.len() - 1 {
                            meta_path.push(segment);
                        } else {
                            meta_path.push(format!("{}.wj.meta", segment));
                        }
                    }
                    candidates.push(meta_path);
                }

                // Try each candidate until we find one that exists.
                // Check .wj-cache/ first, then fall back to colocated (legacy).
                let mut found_metadata = false;
                for candidate in &candidates {
                    let cache_path = if let Some(ref root) = self.source_root {
                        crate::metadata::meta_cache_root(root).join(candidate)
                    } else {
                        std::path::PathBuf::from(".wj-cache").join(candidate)
                    };
                    let legacy_path = if let Some(ref root) = self.source_root {
                        root.join(candidate)
                    } else {
                        candidate.clone()
                    };
                    let full_meta_path = if cache_path.exists() {
                        cache_path
                    } else {
                        legacy_path
                    };

                    if let Ok(meta_json) = std::fs::read_to_string(&full_meta_path) {
                        if let Ok(meta) = serde_json::from_str::<ModuleMetadata>(&meta_json) {
                            // Load all function signatures from metadata
                            for (func_name, sig) in meta.functions {
                                // Convert serialized types back to Type enum
                                let params: Vec<Type> = sig
                                    .params
                                    .iter()
                                    .filter_map(|s| ModuleMetadata::deserialize_type(s))
                                    .collect();

                                let return_type = sig
                                    .return_type
                                    .as_ref()
                                    .and_then(|s| ModuleMetadata::deserialize_type(s));

                                self.function_signatures
                                    .insert(func_name, (params, return_type));
                            }
                            // TDD FIX: Load struct field types for cross-module float inference
                            // Enables LightingConfig { sun_dir_x: -0.5 } to infer f32 from imported struct
                            for (struct_name, fields) in meta.structs {
                                let mut field_map = HashMap::new();
                                for (field_name, type_str) in fields {
                                    if let Some(field_type) =
                                        ModuleMetadata::deserialize_type(&type_str)
                                    {
                                        field_map.insert(field_name, field_type);
                                    }
                                }
                                if !field_map.is_empty() {
                                    self.struct_field_types.insert(struct_name, field_map);
                                }
                            }
                            found_metadata = true;
                            break; // Found and loaded, stop trying candidates
                        }
                    }
                }

                if !found_metadata {}
            }
        }
    }
}

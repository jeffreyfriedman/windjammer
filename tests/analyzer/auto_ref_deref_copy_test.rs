// TDD Test: Auto-ref should NOT add & to dereferenced Copy types
// Bug: contains(*entity) was generating contains(&*entity) instead of contains(*entity)
//
// Expected: If we explicitly dereference a &T to get T (Copy type),
//           we should NOT add & back to it

#[cfg(test)]
mod tests {
    use std::fs;
    use tempfile::TempDir;
    use windjammer::analyzer::Analyzer;
    use windjammer::codegen::CodeGenerator;
    use windjammer::CompilationTarget;

    fn compile_code(source: &str) -> Result<String, String> {
        let temp_dir = TempDir::new().unwrap();
        let source_file = temp_dir.path().join("test.wj");
        let output_dir = temp_dir.path().join("output");
        fs::create_dir_all(&output_dir).unwrap();

        fs::write(&source_file, source).unwrap();

        let source = fs::read_to_string(&source_file).unwrap();
        let mut lexer = windjammer::lexer::Lexer::new(&source);
        let tokens = lexer.tokenize_with_locations();

        let mut parser = windjammer::parser::Parser::new_with_source(
            tokens,
            source_file.to_string_lossy().to_string(),
            source.clone(),
        );
        let program = parser.parse().map_err(|e| format!("Parse error: {}", e))?;

        let mut analyzer = Analyzer::new();
        let (analyzed, signatures, _analyzed_trait_methods) = analyzer
            .analyze_program(&program)
            .map_err(|e| format!("Analysis error: {}", e))?;

        let mut generator = CodeGenerator::new(signatures, CompilationTarget::Rust);

        let rust_code = generator.generate_program(&program, &analyzed);
        Ok(rust_code)
    }

    #[test]
    fn test_deref_copy_no_extra_ref() {
        let source = r#"
@derive(Copy, Clone)
struct Entity {
    id: usize,
}

struct ComponentArray {
    entities: Vec<Entity>,
}

impl ComponentArray {
    fn contains(self, entity: Entity) -> bool {
        self.entities.contains(entity)
    }
}

struct World {
    velocities: ComponentArray,
}

impl World {
    fn update(self) {
        let entities = vec![Entity { id: 5 }]
        // entity is &Entity from iter()
        for entity in entities.iter() {
            // Explicitly deref to get Entity value (Copy)
            // Should NOT add & back to it
            if self.velocities.contains(*entity) {
                println("Has velocity")
            }
        }
    }
}
"#;

        let rust_code = compile_code(source).expect("Failed to compile");
        println!("Generated Rust code:\n{}", rust_code);

        // Check what was actually generated
        if rust_code.contains("contains(&*entity)") {
            panic!(
                "BUG REPRODUCED! Generated contains(&*entity) instead of contains(*entity):\n{}",
                rust_code
            );
        }

        // Should keep the deref as-is
        if !rust_code.contains("contains(*entity)") {
            panic!(
                "Expected contains(*entity) but didn't find it. Generated:\n{}",
                rust_code
            );
        }
    }

    #[test]
    fn test_deref_field_copy_no_extra_ref() {
        let source = r#"
struct Entity {
    id: usize,
}

struct ComponentArray {
    entities: Vec<usize>,
}

impl ComponentArray {
    fn contains(self, entity_id: usize) -> bool {
        self.entities.contains(entity_id)
    }
}

struct World {
    velocities: ComponentArray,
}

impl World {
    fn update(self) {
        let entity_ref: &Entity = &Entity { id: 5 }
        // Deref entity ref, then access field
        if self.velocities.contains((*entity_ref).id) {
            println("Has velocity")
        }
    }
}
"#;

        let rust_code = compile_code(source).expect("Failed to compile");
        println!("Generated Rust code:\n{}", rust_code);

        // Should NOT add & to dereferenced struct's Copy field
        assert!(
            !rust_code.contains("contains(&(*entity_ref).id)"),
            "Should NOT add & to dereferenced struct's Copy field. Generated:\n{}",
            rust_code
        );

        // Should keep as-is or simplify to entity_ref.id
        assert!(
            rust_code.contains("contains((*entity_ref).id)")
                || rust_code.contains("contains(entity_ref.id)"),
            "Should preserve deref access pattern. Generated:\n{}",
            rust_code
        );
    }
}

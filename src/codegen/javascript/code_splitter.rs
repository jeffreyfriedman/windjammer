//! Smart code splitting
//!
//! Splits JavaScript bundles into multiple chunks for optimized loading.

use crate::parser::{Item, Program};
use std::collections::{HashMap, HashSet};

/// Code splitting configuration
#[derive(Debug, Clone)]
pub struct CodeSplitConfig {
    /// Minimum chunk size in bytes
    pub min_chunk_size: usize,
    /// Maximum chunk size in bytes
    pub max_chunk_size: usize,
    /// Enable dynamic imports
    pub dynamic_imports: bool,
}

impl Default for CodeSplitConfig {
    fn default() -> Self {
        Self {
            min_chunk_size: 20_000,  // 20KB minimum
            max_chunk_size: 500_000, // 500KB maximum
            dynamic_imports: true,
        }
    }
}

/// Code chunk
#[derive(Debug, Clone)]
pub struct CodeChunk {
    /// Chunk name
    pub name: String,
    /// Chunk code
    pub code: String,
    /// Dependencies (other chunk names)
    pub dependencies: Vec<String>,
    /// Is this the main entry chunk?
    pub is_entry: bool,
}

/// Code splitter
pub struct CodeSplitter {
    config: CodeSplitConfig,
    chunks: Vec<CodeChunk>,
}

impl CodeSplitter {
    /// Create a new code splitter
    pub fn new(config: CodeSplitConfig) -> Self {
        Self {
            config,
            chunks: Vec::new(),
        }
    }

    /// Split a program into multiple chunks
    pub fn split(&mut self, program: &Program, main_code: &str) -> Vec<CodeChunk> {
        self.chunks.clear();

        // Analyze dependencies
        let dep_graph = self.build_dependency_graph(program);

        // Create main chunk
        let main_chunk = CodeChunk {
            name: "main".to_string(),
            code: main_code.to_string(),
            dependencies: Vec::new(),
            is_entry: true,
        };

        self.chunks.push(main_chunk);

        // Split large modules into separate chunks
        self.split_by_module(program, &dep_graph);

        // Split by dynamic imports if enabled
        if self.config.dynamic_imports {
            self.split_by_dynamic_imports(program);
        }

        self.chunks.clone()
    }

    /// Build dependency graph
    fn build_dependency_graph(&self, program: &Program) -> HashMap<String, HashSet<String>> {
        let mut graph = HashMap::new();

        for item in &program.items {
            if let Item::Function { decl: func, .. } = item {
                let deps = HashSet::new();
                // In a real implementation, we'd analyze function calls
                // For now, just track the function
                graph.insert(func.name.clone(), deps);
            }
        }

        graph
    }

    /// Split by module (group related functions)
    fn split_by_module(
        &mut self,
        program: &Program,
        _dep_graph: &HashMap<String, HashSet<String>>,
    ) {
        let mut current_code = String::new();
        let mut current_size = 0;

        for item in &program.items {
            if let Item::Function { decl: func, .. } = item {
                // Estimate size (rough approximation)
                let func_code = format!("function {}() {{}}\n", func.name);
                let func_size = func_code.len();

                // If adding this function would exceed max chunk size, create a new chunk
                if current_size + func_size > self.config.max_chunk_size && current_size > 0 {
                    if current_size >= self.config.min_chunk_size {
                        self.chunks.push(CodeChunk {
                            name: format!("chunk_{}", self.chunks.len()),
                            code: current_code.clone(),
                            dependencies: vec!["main".to_string()],
                            is_entry: false,
                        });
                    }
                    current_code.clear();
                    current_size = 0;
                }

                current_code.push_str(&func_code);
                current_size += func_size;
            }
        }

        // Add remaining code as a chunk if it meets minimum size
        if current_size >= self.config.min_chunk_size {
            self.chunks.push(CodeChunk {
                name: format!("chunk_{}", self.chunks.len()),
                code: current_code,
                dependencies: vec!["main".to_string()],
                is_entry: false,
            });
        }
    }

    /// Split by dynamic imports
    fn split_by_dynamic_imports(&mut self, _program: &Program) {
        // Identify potential dynamic import points
        // This would analyze async functions and large dependencies
        // For now, this is a placeholder for the pattern
    }

    /// Generate chunk loader code
    pub fn generate_chunk_loader(&self) -> String {
        String::from(
            r#"
// Windjammer Code Splitting Runtime
const __wj_chunks = {};
const __wj_loaded = new Set();

async function __wj_load_chunk(name) {
    if (__wj_loaded.has(name)) {
        return __wj_chunks[name];
    }
    
    try {
        const module = await import(`./${name}.js`);
        __wj_chunks[name] = module;
        __wj_loaded.add(name);
        return module;
    } catch (e) {
        console.error(`Failed to load chunk: ${name}`, e);
        throw e;
    }
}

"#,
        )
    }
}

/// Split code into multiple chunks
pub fn split_code(program: &Program, main_code: &str, config: CodeSplitConfig) -> Vec<CodeChunk> {
    let mut splitter = CodeSplitter::new(config);
    splitter.split(program, main_code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_splitter_creation() {
        let config = CodeSplitConfig::default();
        let splitter = CodeSplitter::new(config);
        assert!(splitter.chunks.is_empty());
    }

    #[test]
    fn test_default_config() {
        let config = CodeSplitConfig::default();
        assert_eq!(config.min_chunk_size, 20_000);
        assert_eq!(config.max_chunk_size, 500_000);
        assert!(config.dynamic_imports);
    }

    #[test]
    fn test_chunk_loader_generation() {
        let config = CodeSplitConfig::default();
        let splitter = CodeSplitter::new(config);
        let loader = splitter.generate_chunk_loader();
        assert!(loader.contains("__wj_load_chunk"));
        assert!(loader.contains("async function"));
    }
}

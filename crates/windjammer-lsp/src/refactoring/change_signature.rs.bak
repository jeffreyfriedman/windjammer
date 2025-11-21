//! Change Function Signature refactoring
//!
//! Allows reordering, adding, and removing function parameters while
//! automatically updating all call sites.

use super::ast_utils;
use crate::database::WindjammerDatabase;
use tower_lsp::lsp_types::*;

/// Change the signature of a function
pub struct ChangeSignature<'a> {
    db: &'a WindjammerDatabase,
    uri: Url,
    position: Position,
}

/// A parameter modification operation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParameterChange {
    /// Add a new parameter
    Add {
        name: String,
        type_hint: Option<String>,
        default_value: String,
        index: usize,
    },
    /// Remove an existing parameter
    Remove { index: usize },
    /// Reorder parameters (old_index -> new_index)
    Reorder { from: usize, to: usize },
    /// Rename a parameter
    Rename { index: usize, new_name: String },
}

/// Result of analyzing a function for signature change
#[derive(Debug, Clone)]
pub struct SignatureAnalysis {
    /// Function name
    pub function_name: String,
    /// Current parameters
    pub parameters: Vec<Parameter>,
    /// Range of the function signature
    pub signature_range: Range,
    /// Call sites that need updating
    pub call_sites: Vec<CallSite>,
    /// Whether it's safe to change
    pub is_safe: bool,
    /// Reason if not safe
    pub unsafe_reason: Option<String>,
}

/// Information about a function parameter
#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub type_hint: Option<String>,
}

/// A call site that needs updating
#[derive(Debug, Clone)]
pub struct CallSite {
    pub range: Range,
    pub arguments: Vec<String>,
}

impl<'a> ChangeSignature<'a> {
    /// Create a new change signature refactoring
    pub fn new(db: &'a WindjammerDatabase, uri: Url, position: Position) -> Self {
        Self { db, uri, position }
    }

    /// Execute the refactoring
    pub fn execute(
        &self,
        changes: &[ParameterChange],
        source: &str,
    ) -> Result<WorkspaceEdit, String> {
        // Step 1: Analyze the function at the cursor
        let analysis = self.analyze_function(source)?;

        // Step 2: Safety checks
        if !analysis.is_safe {
            return Err(analysis
                .unsafe_reason
                .unwrap_or_else(|| "Cannot change signature: unsafe".to_string()));
        }

        // Step 3: Apply changes to get new signature and parameters
        let (new_signature, param_mapping) = self.apply_changes(&analysis, changes)?;

        // Step 4: Create text edits
        let mut edits = vec![];

        // Update function signature
        edits.push(TextEdit {
            range: analysis.signature_range,
            new_text: new_signature,
        });

        // Update all call sites
        for call_site in &analysis.call_sites {
            let new_args = self.reorder_arguments(&call_site.arguments, &param_mapping, changes)?;
            edits.push(TextEdit {
                range: call_site.range,
                new_text: format!("{}({})", analysis.function_name, new_args.join(", ")),
            });
        }

        // Step 5: Create workspace edit
        let mut changes_map = std::collections::HashMap::new();
        changes_map.insert(self.uri.clone(), edits);

        Ok(WorkspaceEdit {
            changes: Some(changes_map),
            document_changes: None,
            change_annotations: None,
        })
    }

    /// Analyze the function at the cursor position
    fn analyze_function(&self, source: &str) -> Result<SignatureAnalysis, String> {
        // Find the function definition at the cursor
        let (function_name, signature_range, parameters) = self.find_function_at_cursor(source)?;

        // Find all call sites
        let call_sites = self.find_call_sites(source, &function_name, signature_range)?;

        // Check if it's safe to change
        let (is_safe, unsafe_reason) = self.check_safety(&function_name, &call_sites);

        Ok(SignatureAnalysis {
            function_name,
            parameters,
            signature_range,
            call_sites,
            is_safe,
            unsafe_reason,
        })
    }

    /// Find the function definition at the cursor
    fn find_function_at_cursor(
        &self,
        source: &str,
    ) -> Result<(String, Range, Vec<Parameter>), String> {
        // Simple regex-based search for function definition
        // Pattern: fn name(param1: type1, param2: type2)
        let pattern = r"fn\s+(\w+)\s*\(([^)]*)\)";
        let re = regex::Regex::new(pattern).map_err(|e| e.to_string())?;

        let cursor_byte = ast_utils::position_to_byte_offset(source, self.position);

        // Find the function that contains the cursor
        for captures in re.captures_iter(source) {
            let full_match = captures.get(0).unwrap();
            let start = full_match.start();
            let end = full_match.end();

            // Check if cursor is within this function definition
            if cursor_byte >= start && cursor_byte <= end {
                let function_name = captures.get(1).unwrap().as_str().to_string();
                let params_str = captures.get(2).unwrap().as_str();

                let start_pos = ast_utils::byte_offset_to_position(source, start);
                let end_pos = ast_utils::byte_offset_to_position(source, end);

                let signature_range = Range {
                    start: start_pos,
                    end: end_pos,
                };

                let parameters = self.parse_parameters(params_str);

                return Ok((function_name, signature_range, parameters));
            }
        }

        Err("No function found at cursor".to_string())
    }

    /// Parse parameter list
    fn parse_parameters(&self, params_str: &str) -> Vec<Parameter> {
        if params_str.trim().is_empty() {
            return vec![];
        }

        params_str
            .split(',')
            .filter_map(|param| {
                let param = param.trim();
                if param.is_empty() {
                    return None;
                }

                // Parse "name: type" or just "name"
                if let Some(colon_pos) = param.find(':') {
                    let name = param[..colon_pos].trim().to_string();
                    let type_hint = Some(param[colon_pos + 1..].trim().to_string());
                    Some(Parameter { name, type_hint })
                } else {
                    Some(Parameter {
                        name: param.to_string(),
                        type_hint: None,
                    })
                }
            })
            .collect()
    }

    /// Find all call sites for the function
    fn find_call_sites(
        &self,
        source: &str,
        function_name: &str,
        signature_range: Range,
    ) -> Result<Vec<CallSite>, String> {
        let mut call_sites = vec![];

        // Pattern: function_name(...)
        let pattern = format!(r"{}\s*\(([^)]*)\)", regex::escape(function_name));
        let re = regex::Regex::new(&pattern).map_err(|e| e.to_string())?;

        let sig_start = ast_utils::position_to_byte_offset(source, signature_range.start);
        let sig_end = ast_utils::position_to_byte_offset(source, signature_range.end);

        for captures in re.captures_iter(source) {
            let full_match = captures.get(0).unwrap();
            let args_match = captures.get(1).unwrap();

            let start = full_match.start();
            let end = full_match.end();

            // Skip the function definition itself
            if start >= sig_start && end <= sig_end {
                continue;
            }

            let start_pos = ast_utils::byte_offset_to_position(source, start);
            let end_pos = ast_utils::byte_offset_to_position(source, end);

            let arguments = self.parse_arguments(args_match.as_str());

            call_sites.push(CallSite {
                range: Range {
                    start: start_pos,
                    end: end_pos,
                },
                arguments,
            });
        }

        Ok(call_sites)
    }

    /// Parse argument list
    fn parse_arguments(&self, args_str: &str) -> Vec<String> {
        if args_str.trim().is_empty() {
            return vec![];
        }

        args_str
            .split(',')
            .map(|arg| arg.trim().to_string())
            .filter(|arg| !arg.is_empty())
            .collect()
    }

    /// Apply parameter changes to generate new signature
    fn apply_changes(
        &self,
        analysis: &SignatureAnalysis,
        changes: &[ParameterChange],
    ) -> Result<(String, Vec<usize>), String> {
        let mut params = analysis.parameters.clone();
        let mut mapping: Vec<usize> = (0..params.len()).collect();

        // Apply changes in order
        for change in changes {
            match change {
                ParameterChange::Add {
                    name,
                    type_hint,
                    index,
                    ..
                } => {
                    if *index > params.len() {
                        return Err(format!("Invalid index: {}", index));
                    }
                    params.insert(
                        *index,
                        Parameter {
                            name: name.clone(),
                            type_hint: type_hint.clone(),
                        },
                    );
                    // Update mapping
                    for m in mapping.iter_mut() {
                        if *m >= *index {
                            *m += 1;
                        }
                    }
                }
                ParameterChange::Remove { index } => {
                    if *index >= params.len() {
                        return Err(format!("Invalid index: {}", index));
                    }
                    params.remove(*index);
                    // Update mapping
                    mapping.retain(|&m| m != *index);
                    for m in mapping.iter_mut() {
                        if *m > *index {
                            *m -= 1;
                        }
                    }
                }
                ParameterChange::Reorder { from, to } => {
                    if *from >= params.len() || *to >= params.len() {
                        return Err(format!("Invalid indices: {} -> {}", from, to));
                    }
                    let param = params.remove(*from);
                    params.insert(*to, param);
                    // Update mapping
                    let old_mapping = mapping[*from];
                    mapping.remove(*from);
                    mapping.insert(*to, old_mapping);
                }
                ParameterChange::Rename { index, new_name } => {
                    if *index >= params.len() {
                        return Err(format!("Invalid index: {}", index));
                    }
                    params[*index].name = new_name.clone();
                }
            }
        }

        // Generate new signature
        let params_str = params
            .iter()
            .map(|p| {
                if let Some(ref type_hint) = p.type_hint {
                    format!("{}: {}", p.name, type_hint)
                } else {
                    p.name.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(", ");

        let new_signature = format!("fn {}({})", analysis.function_name, params_str);

        Ok((new_signature, mapping))
    }

    /// Reorder arguments according to parameter mapping
    fn reorder_arguments(
        &self,
        original_args: &[String],
        mapping: &[usize],
        changes: &[ParameterChange],
    ) -> Result<Vec<String>, String> {
        // Calculate new parameter count after all changes
        let mut param_count = original_args.len();
        for change in changes {
            match change {
                ParameterChange::Add { .. } => param_count += 1,
                ParameterChange::Remove { .. } => param_count -= 1,
                _ => {}
            }
        }

        let mut new_args = vec![String::new(); param_count];

        // First, map existing arguments
        for (new_idx, &old_idx) in mapping.iter().enumerate() {
            if old_idx < original_args.len() && new_idx < new_args.len() {
                new_args[new_idx] = original_args[old_idx].clone();
            }
        }

        // Then handle added parameters with default values
        for change in changes {
            if let ParameterChange::Add {
                index,
                default_value,
                ..
            } = change
            {
                if *index < new_args.len() {
                    new_args[*index] = default_value.clone();
                }
            }
        }

        Ok(new_args)
    }

    /// Check if it's safe to change the signature
    fn check_safety(
        &self,
        _function_name: &str,
        _call_sites: &[CallSite],
    ) -> (bool, Option<String>) {
        // For now, allow all changes
        // TODO: Add more sophisticated safety checks
        (true, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_parameters() {
        let db = WindjammerDatabase::new();
        let uri = Url::parse("file:///test.wj").unwrap();
        let position = Position {
            line: 0,
            character: 0,
        };
        let change_sig = ChangeSignature::new(&db, uri, position);

        let params = change_sig.parse_parameters("x: int, y: string");
        assert_eq!(params.len(), 2);
        assert_eq!(params[0].name, "x");
        assert_eq!(params[0].type_hint, Some("int".to_string()));
        assert_eq!(params[1].name, "y");
        assert_eq!(params[1].type_hint, Some("string".to_string()));
    }

    #[test]
    fn test_parse_arguments() {
        let db = WindjammerDatabase::new();
        let uri = Url::parse("file:///test.wj").unwrap();
        let position = Position {
            line: 0,
            character: 0,
        };
        let change_sig = ChangeSignature::new(&db, uri, position);

        let args = change_sig.parse_arguments("42, \"hello\"");
        assert_eq!(args.len(), 2);
        assert_eq!(args[0], "42");
        assert_eq!(args[1], "\"hello\"");
    }

    #[test]
    fn test_apply_add_parameter() {
        let db = WindjammerDatabase::new();
        let uri = Url::parse("file:///test.wj").unwrap();
        let position = Position {
            line: 0,
            character: 0,
        };
        let change_sig = ChangeSignature::new(&db, uri, position);

        let analysis = SignatureAnalysis {
            function_name: "test".to_string(),
            parameters: vec![Parameter {
                name: "x".to_string(),
                type_hint: Some("int".to_string()),
            }],
            signature_range: Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 0,
                    character: 10,
                },
            },
            call_sites: vec![],
            is_safe: true,
            unsafe_reason: None,
        };

        let changes = vec![ParameterChange::Add {
            name: "y".to_string(),
            type_hint: Some("string".to_string()),
            default_value: "\"default\"".to_string(),
            index: 1,
        }];

        let (new_sig, _) = change_sig.apply_changes(&analysis, &changes).unwrap();
        assert!(new_sig.contains("x: int"));
        assert!(new_sig.contains("y: string"));
    }
}

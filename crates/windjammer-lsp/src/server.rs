#![allow(deprecated)]
use dashmap::DashMap;
use std::sync::{Arc, Mutex};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::analysis::AnalysisDatabase;
use crate::completion::CompletionProvider;
use crate::database::{ParallelConfig, WindjammerDatabase};
use crate::diagnostics::DiagnosticsEngine;
use crate::hover::HoverProvider;
use crate::inlay_hints::InlayHintsProvider;
use crate::refactoring::RefactoringEngine;
use crate::semantic_tokens::SemanticTokensProvider;
use windjammer_lsp::cache::CacheManager;

/// The Windjammer Language Server
///
/// Handles LSP requests and manages the analysis database
pub struct WindjammerLanguageServer {
    client: Client,
    analysis_db: Arc<AnalysisDatabase>,
    /// Salsa incremental computation database (Mutex for Send + Sync)
    salsa_db: Arc<Mutex<WindjammerDatabase>>,
    /// Parallel processing configuration (for future parallel analysis)
    #[allow(dead_code)]
    parallel_config: ParallelConfig,
    /// Persistent disk cache for symbols
    cache_manager: Arc<Mutex<CacheManager>>,
    diagnostics: Arc<DiagnosticsEngine>,
    hover_providers: Arc<Mutex<DashMap<Url, HoverProvider>>>,
    completion_providers: Arc<Mutex<DashMap<Url, CompletionProvider>>>,
    inlay_hints_providers: Arc<Mutex<DashMap<Url, InlayHintsProvider>>>,
    // Note: RefactoringEngine is created on-demand, not stored
    semantic_tokens_providers: Arc<Mutex<DashMap<Url, SemanticTokensProvider>>>,
    /// Map of file URIs to their content
    documents: DashMap<Url, String>,
}

impl WindjammerLanguageServer {
    pub fn new(client: Client) -> Self {
        tracing::info!(
            "Initializing Windjammer Language Server with Salsa incremental computation"
        );

        // Configure parallel processing
        // Use default: all cores, parallel for 5+ files
        let parallel_config = ParallelConfig::default();
        tracing::info!(
            "Parallel processing configured: {} threads, min {} files",
            if parallel_config.num_threads == 0 {
                "all".to_string()
            } else {
                parallel_config.num_threads.to_string()
            },
            parallel_config.min_files_for_parallel
        );

        // Initialize cache manager with default path
        let cache_manager = CacheManager::new(None);
        tracing::info!(
            "Cache manager initialized with {} entries",
            cache_manager.stats().entries
        );

        Self {
            client: client.clone(),
            analysis_db: Arc::new(AnalysisDatabase::new()),
            salsa_db: Arc::new(Mutex::new(WindjammerDatabase::new())),
            parallel_config,
            cache_manager: Arc::new(Mutex::new(cache_manager)),
            diagnostics: Arc::new(DiagnosticsEngine::new(client.clone())),
            hover_providers: Arc::new(Mutex::new(DashMap::new())),
            completion_providers: Arc::new(Mutex::new(DashMap::new())),
            inlay_hints_providers: Arc::new(Mutex::new(DashMap::new())),
            semantic_tokens_providers: Arc::new(Mutex::new(DashMap::new())),
            documents: DashMap::new(),
        }
    }

    /// Analyze a document and publish diagnostics
    async fn analyze_document(&self, uri: Url) {
        if let Some(content) = self.documents.get(&uri) {
            let start = std::time::Instant::now();
            tracing::debug!("Analyzing document: {}", uri);

            // Check cache for this file
            let content_hash = windjammer_lsp::cache::calculate_content_hash(&content);
            let cache_hit = {
                let cache = self.cache_manager.lock().unwrap();
                cache.is_valid(&uri, content_hash)
            };

            if cache_hit {
                tracing::debug!("Cache hit for {} in {:?}", uri, start.elapsed());
            }

            // Get parsed program from Salsa (incremental, memoized)
            // We create the SourceFile and query in one shot to avoid lifetime issues
            let program_owned = {
                let mut db = self.salsa_db.lock().unwrap();
                let source_file = db.set_source_text(uri.clone(), content.clone());
                db.get_program(source_file).clone() // Clone Program to extend lifetime beyond lock
            };

            tracing::debug!(
                "Salsa parse complete in {:?} (memoized: {})",
                start.elapsed(),
                start.elapsed().as_micros() < 100 // < 100μs likely means cache hit
            );

            // Extract symbols and update cache
            {
                let mut db = self.salsa_db.lock().unwrap();
                let source_file = db.set_source_text(uri.clone(), content.clone());
                let symbols = db.get_symbols(source_file);

                // Convert symbols to cached format
                let cached_symbols: Vec<windjammer_lsp::cache::CachedSymbol> = symbols
                    .iter()
                    .map(|s| windjammer_lsp::cache::CachedSymbol {
                        name: s.name.clone(),
                        kind: format!("{:?}", s.kind),
                        line: s.line,
                        character: s.character,
                        type_info: s.type_info.clone(),
                    })
                    .collect();

                // Update cache with new symbols
                let mut cache = self.cache_manager.lock().unwrap();
                let entry = windjammer_lsp::cache::CacheEntry {
                    uri: uri.to_string(),
                    content_hash,
                    modified_time: std::time::SystemTime::now(),
                    symbols: cached_symbols,
                    imports: Vec::new(), // TODO: Extract imports
                };
                cache.insert(uri.clone(), entry);
            }

            // Analyze the file with the old analysis DB (for now)
            // TODO: Eventually migrate analysis to Salsa queries
            let diagnostics = self.analysis_db.analyze_file(&uri, &content);

            // Update providers with Salsa-parsed program
            // Update hover provider
            {
                let mut hover_provider = HoverProvider::new();
                hover_provider.update_program(program_owned.clone());
                let hover_providers = self.hover_providers.lock().unwrap();
                hover_providers.insert(uri.clone(), hover_provider);
            }

            // Update completion provider
            {
                let mut completion_provider = CompletionProvider::new();
                completion_provider.update_program(program_owned.clone());
                let completion_providers = self.completion_providers.lock().unwrap();
                completion_providers.insert(uri.clone(), completion_provider);
            }

            // Note: RefactoringEngine is created on-demand in code_action handler

            // Update semantic tokens provider
            {
                let mut semantic_tokens_provider = SemanticTokensProvider::new();
                semantic_tokens_provider.update_program(program_owned.clone(), content.clone());
                let semantic_tokens_providers = self.semantic_tokens_providers.lock().unwrap();
                semantic_tokens_providers.insert(uri.clone(), semantic_tokens_provider);
            }

            // Update inlay hints provider with ownership analysis
            let analyzed_functions = self.analysis_db.get_analyzed_functions(&uri);
            if !analyzed_functions.is_empty() {
                let mut inlay_hints_provider = InlayHintsProvider::new();
                inlay_hints_provider.update_analyzed_functions(analyzed_functions);
                let inlay_hints_providers = self.inlay_hints_providers.lock().unwrap();
                inlay_hints_providers.insert(uri.clone(), inlay_hints_provider);
            }

            // Publish diagnostics to the client
            self.diagnostics.publish(&uri, diagnostics).await;
        }
    }

    /// Process multiple files in parallel using the configured parallel processing
    ///
    /// This is useful for workspace-wide operations like "find all references"
    /// or initial workspace indexing.
    #[allow(dead_code)] // Planned for future parallel analysis
    async fn process_files_parallel(&self, uris: Vec<Url>) {
        if uris.is_empty() {
            return;
        }

        let start = std::time::Instant::now();
        tracing::info!("Processing {} files in parallel", uris.len());

        // Collect all (uri, text) pairs
        let files: Vec<(Url, String)> = uris
            .iter()
            .filter_map(|uri| {
                self.documents
                    .get(uri)
                    .map(|content| (uri.clone(), content.clone()))
            })
            .collect();

        if files.is_empty() {
            tracing::warn!("No documents found to process");
            return;
        }

        // Process files in parallel using Salsa database
        {
            let mut db = self.salsa_db.lock().unwrap();
            let _source_files = db.process_files_parallel(files, &self.parallel_config);
        }

        tracing::info!(
            "Processed {} files in parallel in {:?}",
            uris.len(),
            start.elapsed()
        );
    }

    /// Helper to convert Type to string for display
    #[allow(clippy::only_used_in_recursion)]
    fn type_to_string(&self, ty: &windjammer::parser::Type) -> String {
        use windjammer::parser::Type;
        match ty {
            Type::Int => "int".to_string(),
            Type::Int32 => "i32".to_string(),
            Type::Uint => "uint".to_string(),
            Type::Float => "float".to_string(),
            Type::Bool => "bool".to_string(),
            Type::String => "string".to_string(),
            Type::Custom(name) => name.clone(),
            Type::Generic(name) => name.clone(),
            Type::Parameterized(base, params) => {
                let params_str = params
                    .iter()
                    .map(|p| self.type_to_string(p))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}<{}>", base, params_str)
            }
            Type::Associated(base, assoc) => format!("{}::{}", base, assoc),
            Type::TraitObject(name) => format!("dyn {}", name),
            Type::Option(inner) => format!("Option<{}>", self.type_to_string(inner)),
            Type::Result(ok, err) => {
                format!(
                    "Result<{}, {}>",
                    self.type_to_string(ok),
                    self.type_to_string(err)
                )
            }
            Type::Vec(inner) => format!("Vec<{}>", self.type_to_string(inner)),
            Type::Array(inner, size) => format!("[{}; {}]", self.type_to_string(inner), size),
            Type::Reference(inner) => format!("&{}", self.type_to_string(inner)),
            Type::MutableReference(inner) => format!("&mut {}", self.type_to_string(inner)),
            Type::Tuple(types) => {
                let types_str = types
                    .iter()
                    .map(|t| self.type_to_string(t))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("({})", types_str)
            }
            Type::Infer => "_".to_string(),
        }
    }

    /// Get the word at a given position in a document
    /// TODO: Improve this with proper lexical analysis
    fn get_word_at_position(&self, uri: &Url, position: Position) -> Option<String> {
        let content = self.documents.get(uri)?;
        let lines: Vec<&str> = content.value().split('\n').collect();

        if position.line as usize >= lines.len() {
            return None;
        }

        let line = lines[position.line as usize];
        let char_pos = position.character as usize;

        if char_pos >= line.len() {
            return None;
        }

        // Find word boundaries
        let mut start = char_pos;
        let mut end = char_pos;

        let chars: Vec<char> = line.chars().collect();

        // Find start of word
        while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_') {
            start -= 1;
        }

        // Find end of word
        while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
            end += 1;
        }

        if start < end {
            Some(chars[start..end].iter().collect())
        } else {
            None
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for WindjammerLanguageServer {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        tracing::info!("Client initialized with params: {:?}", params.capabilities);

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                // Text synchronization
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::FULL),
                        will_save: None,
                        will_save_wait_until: None,
                        save: Some(TextDocumentSyncSaveOptions::SaveOptions(SaveOptions {
                            include_text: Some(true),
                        })),
                    },
                )),

                // Hover information
                hover_provider: Some(HoverProviderCapability::Simple(true)),

                // Completion
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![
                        ".".to_string(),
                        ":".to_string(),
                        "@".to_string(),
                    ]),
                    ..Default::default()
                }),

                // Signature help (parameter hints)
                signature_help_provider: Some(SignatureHelpOptions {
                    trigger_characters: Some(vec!["(".to_string(), ",".to_string()]),
                    retrigger_characters: None,
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                }),

                // Go to definition
                definition_provider: Some(OneOf::Left(true)),

                // Find references
                references_provider: Some(OneOf::Left(true)),

                // Rename
                rename_provider: Some(OneOf::Left(true)),

                // Document symbols (outline)
                document_symbol_provider: Some(OneOf::Left(true)),

                // Workspace symbols (search)
                workspace_symbol_provider: Some(OneOf::Left(true)),

                // Code actions (quick fixes)
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),

                // Formatting
                document_formatting_provider: Some(OneOf::Left(true)),

                // Inlay hints (ownership annotations)
                inlay_hint_provider: Some(OneOf::Left(true)),

                // Semantic tokens (enhanced syntax highlighting)
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            legend: crate::semantic_tokens::get_semantic_token_legend(),
                            range: Some(false),
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                            ..Default::default()
                        },
                    ),
                ),

                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "windjammer-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        tracing::info!("Server initialized successfully");

        self.client
            .log_message(MessageType::INFO, "Windjammer LSP server initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        tracing::info!("Shutdown request received");

        // Save cache to disk before shutting down
        {
            let cache = self.cache_manager.lock().unwrap();
            if let Err(e) = cache.save_to_disk() {
                tracing::warn!("Failed to save cache on shutdown: {}", e);
            } else {
                tracing::info!("Cache saved successfully");
            }
        }

        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        tracing::debug!("Document opened: {}", params.text_document.uri);

        // Store the document content
        self.documents
            .insert(params.text_document.uri.clone(), params.text_document.text);

        // Analyze the document
        self.analyze_document(params.text_document.uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let start = std::time::Instant::now();
        tracing::debug!("Document changed: {}", params.text_document.uri);

        // Update the document content (we use FULL sync, so just take the first change)
        if let Some(change) = params.content_changes.into_iter().next() {
            self.documents
                .insert(params.text_document.uri.clone(), change.text.clone());

            tracing::debug!("Document content updated in {:?}", start.elapsed());

            // Re-analyze the document (Salsa will handle incremental recomputation)
            self.analyze_document(params.text_document.uri).await;
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        tracing::debug!("Document saved: {}", params.text_document.uri);

        // Update content if provided
        if let Some(text) = params.text {
            self.documents
                .insert(params.text_document.uri.clone(), text);
        }

        // Re-analyze the document
        self.analyze_document(params.text_document.uri).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        tracing::debug!("Document closed: {}", params.text_document.uri);

        // Remove the document
        self.documents.remove(&params.text_document.uri);

        // Note: Salsa will automatically GC unused data

        // Clear diagnostics
        self.diagnostics.clear(&params.text_document.uri).await;
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let position = params.text_document_position_params.position;

        tracing::debug!("Hover request: {} at {:?}", uri, position);

        // Get the hover provider for this file
        let providers = self.hover_providers.lock().unwrap();
        let result = providers
            .get(&uri)
            .and_then(|provider| provider.get_hover(position));
        drop(providers); // Explicitly drop the lock

        Ok(result)
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri.clone();
        let position = params.text_document_position.position;

        tracing::debug!("Completion request: {} at {:?}", uri, position);

        // Get the completion provider for this file
        let providers = self.completion_providers.lock().unwrap();
        let result = providers
            .get(&uri)
            .and_then(|provider| provider.get_completions(position));
        drop(providers); // Explicitly drop the lock

        Ok(result)
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let position = params.text_document_position_params.position;

        tracing::debug!("Go to definition (cross-file): {} at {:?}", uri, position);

        // Get the word at the cursor position
        let symbol_name = self.get_word_at_position(&uri, position);

        if let Some(name) = symbol_name {
            // Use Salsa to find definition across all open files
            let location = {
                let mut db = self.salsa_db.lock().unwrap();

                // Collect all open files as SourceFiles
                let files: Vec<_> = self
                    .documents
                    .iter()
                    .map(|entry| {
                        let file_uri = entry.key().clone();
                        let text = entry.value().clone();
                        db.set_source_text(file_uri, text)
                    })
                    .collect();

                // Find definition across all files
                db.find_definition(&name, &files)
            }; // Lock released

            if let Some(loc) = location {
                tracing::debug!(
                    "Found definition for '{}' in {} (cross-file)",
                    name,
                    loc.uri
                );
                return Ok(Some(GotoDefinitionResponse::Scalar(loc)));
            }

            // Fallback to old single-file search
            if let Some(symbol_table) = self.analysis_db.get_symbol_table(&uri) {
                if let Some(symbol_def) = symbol_table.find_symbol(&name) {
                    tracing::debug!(
                        "Found definition for '{}' at {:?} (fallback)",
                        name,
                        symbol_def.location
                    );
                    return Ok(Some(GotoDefinitionResponse::Scalar(
                        symbol_def.location.clone(),
                    )));
                }
            }
        }

        Ok(None)
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri.clone();
        let position = params.text_document_position.position;

        tracing::debug!("Find references (cross-file): {} at {:?}", uri, position);

        // Get the word at the cursor position
        let symbol_name = self.get_word_at_position(&uri, position);

        if let Some(name) = symbol_name {
            // Use Salsa to find references across all open files
            let locations = {
                let mut db = self.salsa_db.lock().unwrap();

                // Collect all open files as SourceFiles
                let files: Vec<_> = self
                    .documents
                    .iter()
                    .map(|entry| {
                        let file_uri = entry.key().clone();
                        let text = entry.value().clone();
                        db.set_source_text(file_uri, text)
                    })
                    .collect();

                // Find all references across all files
                db.find_all_references(&name, &files)
            }; // Lock released

            if !locations.is_empty() {
                tracing::debug!(
                    "Found {} references to '{}' across {} files",
                    locations.len(),
                    name,
                    self.documents.len()
                );
                return Ok(Some(locations));
            }

            // Fallback to old single-file search if Salsa finds nothing
            if let Some(symbol_table) = self.analysis_db.get_symbol_table(&uri) {
                let refs = symbol_table.find_references(&name);

                if !refs.is_empty() {
                    let mut locations: Vec<Location> =
                        refs.iter().map(|r| r.location.clone()).collect();

                    if params.context.include_declaration {
                        if let Some(symbol_def) = symbol_table.find_symbol(&name) {
                            locations.push(symbol_def.location.clone());
                        }
                    }

                    tracing::debug!(
                        "Found {} references to '{}' (fallback)",
                        locations.len(),
                        name
                    );
                    return Ok(Some(locations));
                }
            }
        }

        Ok(None)
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let uri = params.text_document_position.text_document.uri.clone();
        let position = params.text_document_position.position;
        let new_name = params.new_name.clone();

        tracing::debug!(
            "Rename (cross-file): {} at {:?} to {}",
            uri,
            position,
            new_name
        );

        // Get the word at the cursor position
        let symbol_name = self.get_word_at_position(&uri, position);

        if let Some(old_name) = symbol_name {
            // Use Salsa to find all references across all open files
            let locations = {
                let mut db = self.salsa_db.lock().unwrap();

                // Collect all open files as SourceFiles
                let files: Vec<_> = self
                    .documents
                    .iter()
                    .map(|entry| {
                        let file_uri = entry.key().clone();
                        let text = entry.value().clone();
                        db.set_source_text(file_uri, text)
                    })
                    .collect();

                // Find all references across all files (includes definitions)
                db.find_all_references(&old_name, &files)
            }; // Lock released

            if !locations.is_empty() {
                use std::collections::HashMap;
                let mut changes: HashMap<Url, Vec<TextEdit>> = HashMap::new();

                // Create text edits for all locations
                for location in locations {
                    let text_edit = TextEdit {
                        range: location.range,
                        new_text: new_name.clone(),
                    };

                    changes.entry(location.uri).or_default().push(text_edit);
                }

                let num_files = changes.len();
                let num_edits: usize = changes.values().map(|v| v.len()).sum();

                tracing::debug!(
                    "Renaming '{}' to '{}' with {} edits across {} files (cross-file)",
                    old_name,
                    new_name,
                    num_edits,
                    num_files
                );

                return Ok(Some(WorkspaceEdit {
                    changes: Some(changes),
                    document_changes: None,
                    change_annotations: None,
                }));
            }

            // Fallback to old single-file rename
            if let Some(symbol_table) = self.analysis_db.get_symbol_table(&uri) {
                let refs = symbol_table.find_references(&old_name);

                if !refs.is_empty() || symbol_table.find_symbol(&old_name).is_some() {
                    use std::collections::HashMap;
                    let mut changes: HashMap<Url, Vec<TextEdit>> = HashMap::new();

                    // Create text edits for all references
                    for reference in refs {
                        let text_edit = TextEdit {
                            range: reference.location.range,
                            new_text: new_name.clone(),
                        };

                        changes
                            .entry(reference.location.uri.clone())
                            .or_default()
                            .push(text_edit);
                    }

                    // Also rename the definition
                    if let Some(symbol_def) = symbol_table.find_symbol(&old_name) {
                        let text_edit = TextEdit {
                            range: symbol_def.location.range,
                            new_text: new_name.clone(),
                        };

                        changes
                            .entry(symbol_def.location.uri.clone())
                            .or_default()
                            .push(text_edit);
                    }

                    if !changes.is_empty() {
                        tracing::debug!(
                            "Renaming '{}' to '{}' with {} edits across {} files (fallback)",
                            old_name,
                            new_name,
                            changes.values().map(|v| v.len()).sum::<usize>(),
                            changes.len()
                        );

                        return Ok(Some(WorkspaceEdit {
                            changes: Some(changes),
                            document_changes: None,
                            change_annotations: None,
                        }));
                    }
                }
            }
        }

        Ok(None)
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri.clone();
        tracing::debug!("Document symbol: {}", uri);

        // Get the program for this file
        let program = match self.analysis_db.get_program(&uri) {
            Some(prog) => prog,
            None => return Ok(None),
        };

        let mut symbols = Vec::new();

        // Collect symbols from all items
        for item in &program.items {
            match item {
                windjammer::parser::Item::Function(func) => {
                    symbols.push(SymbolInformation {
                        name: func.name.clone(),
                        kind: SymbolKind::FUNCTION,
                        tags: None,
                        deprecated: None,
                        location: Location {
                            uri: uri.clone(),
                            range: Range {
                                start: Position {
                                    line: 0,
                                    character: 0,
                                },
                                end: Position {
                                    line: 0,
                                    character: 0,
                                },
                            },
                        },
                        container_name: None,
                    });
                }
                windjammer::parser::Item::Struct(s) => {
                    symbols.push(SymbolInformation {
                        name: s.name.clone(),
                        kind: SymbolKind::STRUCT,
                        tags: None,
                        deprecated: None,
                        location: Location {
                            uri: uri.clone(),
                            range: Range {
                                start: Position {
                                    line: 0,
                                    character: 0,
                                },
                                end: Position {
                                    line: 0,
                                    character: 0,
                                },
                            },
                        },
                        container_name: None,
                    });

                    // Add struct fields
                    for field in &s.fields {
                        symbols.push(SymbolInformation {
                            name: field.name.clone(),
                            kind: SymbolKind::FIELD,
                            tags: None,
                            deprecated: None,
                            location: Location {
                                uri: uri.clone(),
                                range: Range {
                                    start: Position {
                                        line: 0,
                                        character: 0,
                                    },
                                    end: Position {
                                        line: 0,
                                        character: 0,
                                    },
                                },
                            },
                            container_name: Some(s.name.clone()),
                        });
                    }
                }
                windjammer::parser::Item::Enum(e) => {
                    symbols.push(SymbolInformation {
                        name: e.name.clone(),
                        kind: SymbolKind::ENUM,
                        tags: None,
                        deprecated: None,
                        location: Location {
                            uri: uri.clone(),
                            range: Range {
                                start: Position {
                                    line: 0,
                                    character: 0,
                                },
                                end: Position {
                                    line: 0,
                                    character: 0,
                                },
                            },
                        },
                        container_name: None,
                    });

                    // Add enum variants
                    for variant in &e.variants {
                        symbols.push(SymbolInformation {
                            name: variant.name.clone(),
                            kind: SymbolKind::ENUM_MEMBER,
                            tags: None,
                            deprecated: None,
                            location: Location {
                                uri: uri.clone(),
                                range: Range {
                                    start: Position {
                                        line: 0,
                                        character: 0,
                                    },
                                    end: Position {
                                        line: 0,
                                        character: 0,
                                    },
                                },
                            },
                            container_name: Some(e.name.clone()),
                        });
                    }
                }
                windjammer::parser::Item::Trait(t) => {
                    symbols.push(SymbolInformation {
                        name: t.name.clone(),
                        kind: SymbolKind::INTERFACE,
                        tags: None,
                        deprecated: None,
                        location: Location {
                            uri: uri.clone(),
                            range: Range {
                                start: Position {
                                    line: 0,
                                    character: 0,
                                },
                                end: Position {
                                    line: 0,
                                    character: 0,
                                },
                            },
                        },
                        container_name: None,
                    });
                }
                windjammer::parser::Item::Impl(impl_block) => {
                    // Add impl block methods
                    for method in &impl_block.functions {
                        symbols.push(SymbolInformation {
                            name: method.name.clone(),
                            kind: SymbolKind::METHOD,
                            tags: None,
                            deprecated: None,
                            location: Location {
                                uri: uri.clone(),
                                range: Range {
                                    start: Position {
                                        line: 0,
                                        character: 0,
                                    },
                                    end: Position {
                                        line: 0,
                                        character: 0,
                                    },
                                },
                            },
                            container_name: Some(impl_block.type_name.clone()),
                        });
                    }
                }
                windjammer::parser::Item::Static { name, .. } => {
                    symbols.push(SymbolInformation {
                        name: name.clone(),
                        kind: SymbolKind::CONSTANT,
                        tags: None,
                        deprecated: None,
                        location: Location {
                            uri: uri.clone(),
                            range: Range {
                                start: Position {
                                    line: 0,
                                    character: 0,
                                },
                                end: Position {
                                    line: 0,
                                    character: 0,
                                },
                            },
                        },
                        container_name: None,
                    });
                }
                _ => {}
            }
        }

        if symbols.is_empty() {
            Ok(None)
        } else {
            Ok(Some(DocumentSymbolResponse::Flat(symbols)))
        }
    }

    async fn symbol(
        &self,
        params: WorkspaceSymbolParams,
    ) -> Result<Option<Vec<SymbolInformation>>> {
        let query = params.query.to_lowercase();
        tracing::debug!("Workspace symbol search: {}", query);

        let mut results = Vec::new();

        // Search across all documents in the workspace
        for entry in self.documents.iter() {
            let uri = entry.key().clone();

            // Get the program for this file
            if let Some(program) = self.analysis_db.get_program(&uri) {
                // Search through all items
                for item in &program.items {
                    match item {
                        windjammer::parser::Item::Function(func) => {
                            if func.name.to_lowercase().contains(&query) {
                                results.push(SymbolInformation {
                                    name: func.name.clone(),
                                    kind: SymbolKind::FUNCTION,
                                    tags: None,
                                    deprecated: None,
                                    location: Location {
                                        uri: uri.clone(),
                                        range: Range {
                                            start: Position {
                                                line: 0,
                                                character: 0,
                                            },
                                            end: Position {
                                                line: 0,
                                                character: 0,
                                            },
                                        },
                                    },
                                    container_name: None,
                                });
                            }
                        }
                        windjammer::parser::Item::Struct(s) => {
                            if s.name.to_lowercase().contains(&query) {
                                results.push(SymbolInformation {
                                    name: s.name.clone(),
                                    kind: SymbolKind::STRUCT,
                                    tags: None,
                                    deprecated: None,
                                    location: Location {
                                        uri: uri.clone(),
                                        range: Range {
                                            start: Position {
                                                line: 0,
                                                character: 0,
                                            },
                                            end: Position {
                                                line: 0,
                                                character: 0,
                                            },
                                        },
                                    },
                                    container_name: None,
                                });
                            }
                        }
                        windjammer::parser::Item::Enum(e) => {
                            if e.name.to_lowercase().contains(&query) {
                                results.push(SymbolInformation {
                                    name: e.name.clone(),
                                    kind: SymbolKind::ENUM,
                                    tags: None,
                                    deprecated: None,
                                    location: Location {
                                        uri: uri.clone(),
                                        range: Range {
                                            start: Position {
                                                line: 0,
                                                character: 0,
                                            },
                                            end: Position {
                                                line: 0,
                                                character: 0,
                                            },
                                        },
                                    },
                                    container_name: None,
                                });
                            }
                        }
                        windjammer::parser::Item::Trait(t) => {
                            if t.name.to_lowercase().contains(&query) {
                                results.push(SymbolInformation {
                                    name: t.name.clone(),
                                    kind: SymbolKind::INTERFACE,
                                    tags: None,
                                    deprecated: None,
                                    location: Location {
                                        uri: uri.clone(),
                                        range: Range {
                                            start: Position {
                                                line: 0,
                                                character: 0,
                                            },
                                            end: Position {
                                                line: 0,
                                                character: 0,
                                            },
                                        },
                                    },
                                    container_name: None,
                                });
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        if results.is_empty() {
            Ok(None)
        } else {
            Ok(Some(results))
        }
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let uri = params.text_document.uri.clone();
        let range = params.range;

        tracing::debug!("Code action request: {} for range {:?}", uri, range);

        // Create refactoring engine on-demand
        let db_lock = self.salsa_db.lock().unwrap();
        let engine = RefactoringEngine::new(&db_lock);

        let context = CodeActionContext {
            diagnostics: vec![],
            only: None,
            trigger_kind: None,
        };
        let actions = engine.code_actions(&uri, range, &context);

        if !actions.is_empty() {
            return Ok(Some(actions));
        }

        // TODO: Implement more code actions
        // - Add missing import
        // - Implement missing trait methods
        // - Fix ownership annotations

        Ok(None)
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        tracing::debug!("Format document: {}", params.text_document.uri);

        // TODO: Integrate with `wj fmt`
        // - Format the document
        // - Return text edits

        Ok(None)
    }

    async fn inlay_hint(&self, params: InlayHintParams) -> Result<Option<Vec<InlayHint>>> {
        let uri = params.text_document.uri.clone();
        let range = params.range;

        tracing::debug!("Inlay hint request: {} for range {:?}", uri, range);

        // Get the inlay hints provider for this file
        let providers = self.inlay_hints_providers.lock().unwrap();
        let result = providers
            .get(&uri)
            .map(|provider| provider.get_inlay_hints(range));
        drop(providers); // Explicitly drop the lock

        Ok(result)
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri.clone();
        tracing::debug!("Semantic tokens request: {}", uri);

        // Get the semantic tokens provider for this file
        let providers = self.semantic_tokens_providers.lock().unwrap();
        let tokens = providers
            .get(&uri)
            .and_then(|provider| provider.get_semantic_tokens());
        drop(providers); // Explicitly drop the lock

        match tokens {
            Some(tokens) => Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
                result_id: None,
                data: tokens,
            }))),
            None => Ok(None),
        }
    }

    async fn signature_help(&self, params: SignatureHelpParams) -> Result<Option<SignatureHelp>> {
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let position = params.text_document_position_params.position;
        tracing::debug!("Signature help request: {} at {:?}", uri, position);

        // Get document content
        let content = match self.documents.get(&uri) {
            Some(content) => content.clone(),
            None => return Ok(None),
        };

        // Get the program for this file
        let program = match self.analysis_db.get_program(&uri) {
            Some(prog) => prog,
            None => return Ok(None),
        };

        // Find function at cursor position (simplified - look for function name before cursor)
        let line_start = content
            .lines()
            .take(position.line as usize)
            .map(|l| l.len() + 1)
            .sum::<usize>();
        let cursor_offset = line_start + position.character as usize;

        // Look backward for function call pattern
        let before_cursor = &content[..cursor_offset];

        // Find the last opening parenthesis
        if let Some(paren_pos) = before_cursor.rfind('(') {
            // Extract function name before the parenthesis
            let before_paren = &before_cursor[..paren_pos].trim_end();

            // Simple heuristic: function name is the last word before '('
            let func_name = before_paren
                .split(|c: char| !c.is_alphanumeric() && c != '_')
                .next_back()
                .unwrap_or("")
                .to_string();

            if !func_name.is_empty() {
                // Find this function in the program
                for item in &program.items {
                    if let windjammer::parser::Item::Function(_func_decl) = item {
                        // Get analyzed function for this declaration
                        let analyzed_funcs = self.analysis_db.get_analyzed_functions(&uri);
                        let func = analyzed_funcs.iter().find(|f| f.decl.name == func_name);

                        if let Some(func) = func {
                            // Build signature string
                            let params_str: Vec<String> = func
                                .decl
                                .parameters
                                .iter()
                                .map(|p| format!("{}: {}", p.name, self.type_to_string(&p.type_)))
                                .collect();

                            let return_str = func
                                .decl
                                .return_type
                                .as_ref()
                                .map(|t| format!(" -> {}", self.type_to_string(t)))
                                .unwrap_or_default();

                            let signature_label = format!(
                                "fn {}({}){}",
                                func_name,
                                params_str.join(", "),
                                return_str
                            );

                            // Calculate active parameter based on comma count
                            let params_section = &before_cursor[paren_pos + 1..];
                            let comma_count = params_section.matches(',').count();
                            let active_param =
                                comma_count.min(func.decl.parameters.len().saturating_sub(1));

                            // Build parameter information
                            let parameters: Vec<ParameterInformation> = func
                                .decl
                                .parameters
                                .iter()
                                .map(|p| ParameterInformation {
                                    label: ParameterLabel::Simple(format!(
                                        "{}: {}",
                                        p.name,
                                        self.type_to_string(&p.type_)
                                    )),
                                    documentation: None,
                                })
                                .collect();

                            return Ok(Some(SignatureHelp {
                                signatures: vec![SignatureInformation {
                                    label: signature_label,
                                    documentation: None,
                                    parameters: Some(parameters),
                                    active_parameter: Some(active_param as u32),
                                }],
                                active_signature: Some(0),
                                active_parameter: Some(active_param as u32),
                            }));
                        }
                    }
                }
            }
        }

        Ok(None)
    }
}

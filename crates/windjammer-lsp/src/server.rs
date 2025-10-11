use dashmap::DashMap;
use std::sync::{Arc, RwLock};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::analysis::AnalysisDatabase;
use crate::completion::CompletionProvider;
use crate::diagnostics::DiagnosticsEngine;
use crate::hover::HoverProvider;
use crate::inlay_hints::InlayHintsProvider;

/// The Windjammer Language Server
///
/// Handles LSP requests and manages the analysis database
pub struct WindjammerLanguageServer {
    client: Client,
    analysis_db: Arc<AnalysisDatabase>,
    diagnostics: Arc<DiagnosticsEngine>,
    hover_providers: Arc<RwLock<DashMap<Url, HoverProvider>>>,
    completion_providers: Arc<RwLock<DashMap<Url, CompletionProvider>>>,
    inlay_hints_providers: Arc<RwLock<DashMap<Url, InlayHintsProvider>>>,
    /// Map of file URIs to their content
    documents: DashMap<Url, String>,
}

impl WindjammerLanguageServer {
    pub fn new(client: Client) -> Self {
        tracing::info!("Initializing Windjammer Language Server");

        Self {
            client: client.clone(),
            analysis_db: Arc::new(AnalysisDatabase::new()),
            diagnostics: Arc::new(DiagnosticsEngine::new(client.clone())),
            hover_providers: Arc::new(RwLock::new(DashMap::new())),
            completion_providers: Arc::new(RwLock::new(DashMap::new())),
            inlay_hints_providers: Arc::new(RwLock::new(DashMap::new())),
            documents: DashMap::new(),
        }
    }

    /// Analyze a document and publish diagnostics
    async fn analyze_document(&self, uri: Url) {
        if let Some(content) = self.documents.get(&uri) {
            tracing::debug!("Analyzing document: {}", uri);

            // Analyze the file
            let diagnostics = self.analysis_db.analyze_file(&uri, &content);

            // Update providers with parsed program and analysis results
            if let Some(program) = self.analysis_db.get_program(&uri) {
                // Update hover provider
                let mut hover_provider = HoverProvider::new();
                hover_provider.update_program(program.clone());
                let hover_providers = self.hover_providers.write().unwrap();
                hover_providers.insert(uri.clone(), hover_provider);

                // Update completion provider
                let mut completion_provider = CompletionProvider::new();
                completion_provider.update_program(program);
                let completion_providers = self.completion_providers.write().unwrap();
                completion_providers.insert(uri.clone(), completion_provider);
            }

            // Update inlay hints provider with ownership analysis
            let analyzed_functions = self.analysis_db.get_analyzed_functions(&uri);
            if !analyzed_functions.is_empty() {
                let mut inlay_hints_provider = InlayHintsProvider::new();
                inlay_hints_provider.update_analyzed_functions(analyzed_functions);
                let inlay_hints_providers = self.inlay_hints_providers.write().unwrap();
                inlay_hints_providers.insert(uri.clone(), inlay_hints_provider);
            }

            // Publish diagnostics to the client
            self.diagnostics.publish(&uri, diagnostics).await;
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
                            legend: SemanticTokensLegend {
                                token_types: vec![
                                    SemanticTokenType::NAMESPACE,
                                    SemanticTokenType::TYPE,
                                    SemanticTokenType::CLASS,
                                    SemanticTokenType::ENUM,
                                    SemanticTokenType::INTERFACE,
                                    SemanticTokenType::STRUCT,
                                    SemanticTokenType::TYPE_PARAMETER,
                                    SemanticTokenType::PARAMETER,
                                    SemanticTokenType::VARIABLE,
                                    SemanticTokenType::PROPERTY,
                                    SemanticTokenType::ENUM_MEMBER,
                                    SemanticTokenType::FUNCTION,
                                    SemanticTokenType::METHOD,
                                    SemanticTokenType::MACRO,
                                    SemanticTokenType::KEYWORD,
                                    SemanticTokenType::COMMENT,
                                    SemanticTokenType::STRING,
                                    SemanticTokenType::NUMBER,
                                    SemanticTokenType::OPERATOR,
                                ],
                                token_modifiers: vec![
                                    SemanticTokenModifier::DECLARATION,
                                    SemanticTokenModifier::DEFINITION,
                                    SemanticTokenModifier::READONLY,
                                    SemanticTokenModifier::STATIC,
                                    SemanticTokenModifier::DEPRECATED,
                                    SemanticTokenModifier::ABSTRACT,
                                    SemanticTokenModifier::ASYNC,
                                    SemanticTokenModifier::MODIFICATION,
                                    SemanticTokenModifier::DOCUMENTATION,
                                ],
                            },
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
        tracing::debug!("Document changed: {}", params.text_document.uri);

        // Update the document content (we use FULL sync, so just take the first change)
        if let Some(change) = params.content_changes.into_iter().next() {
            self.documents
                .insert(params.text_document.uri.clone(), change.text);

            // Re-analyze the document
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
        let providers = self.hover_providers.read().unwrap();
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
        let providers = self.completion_providers.read().unwrap();
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
        tracing::debug!(
            "Go to definition: {} at {:?}",
            params.text_document_position_params.text_document.uri,
            params.text_document_position_params.position
        );

        // TODO: Implement go to definition
        // - Navigate to function, type, trait definitions
        // - Navigate to module definitions

        Ok(None)
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        tracing::debug!(
            "Find references: {} at {:?}",
            params.text_document_position.text_document.uri,
            params.text_document_position.position
        );

        // TODO: Implement find references
        // - Find all usages of a symbol
        // - Cross-file search

        Ok(None)
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        tracing::debug!(
            "Rename: {} at {:?} to {}",
            params.text_document_position.text_document.uri,
            params.text_document_position.position,
            params.new_name
        );

        // TODO: Implement rename
        // - Safe rename across all files
        // - Update imports and references

        Ok(None)
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        tracing::debug!("Document symbol: {}", params.text_document.uri);

        // TODO: Implement document symbols (outline)
        // - List all functions, structs, enums, etc.

        Ok(None)
    }

    async fn symbol(
        &self,
        params: WorkspaceSymbolParams,
    ) -> Result<Option<Vec<SymbolInformation>>> {
        tracing::debug!("Workspace symbol: {}", params.query);

        // TODO: Implement workspace symbols (search)
        // - Search for symbols across entire workspace

        Ok(None)
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        tracing::debug!("Code action: {}", params.text_document.uri);

        // TODO: Implement code actions (quick fixes)
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
        let providers = self.inlay_hints_providers.read().unwrap();
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
        tracing::debug!("Semantic tokens: {}", params.text_document.uri);

        // TODO: Implement semantic tokens
        // - Enhanced syntax highlighting

        Ok(None)
    }
}

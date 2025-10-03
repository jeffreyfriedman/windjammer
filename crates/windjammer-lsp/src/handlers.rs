use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};
use dashmap::DashMap;
use std::sync::Arc;

use crate::database::{RootDatabase, FileId, SourceDatabase};
use crate::diagnostics::publish_diagnostics;
use crate::completion::get_completions;
use crate::hover::get_hover_info;

pub struct Backend {
    client: Client,
    db: Arc<RootDatabase>,
    file_map: Arc<DashMap<Url, FileId>>,
    next_file_id: Arc<std::sync::atomic::AtomicU32>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Backend {
            client,
            db: Arc::new(RootDatabase::default()),
            file_map: Arc::new(DashMap::new()),
            next_file_id: Arc::new(std::sync::atomic::AtomicU32::new(0)),
        }
    }
    
    fn get_or_create_file_id(&self, uri: &Url) -> FileId {
        if let Some(entry) = self.file_map.get(uri) {
            *entry
        } else {
            let id = FileId(
                self.next_file_id
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst),
            );
            self.file_map.insert(uri.clone(), id);
            id
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![".".to_string(), "@".to_string()]),
                    ..Default::default()
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                workspace_symbol_provider: Some(OneOf::Left(true)),
                diagnostic_provider: Some(DiagnosticServerCapabilities::Options(
                    DiagnosticOptions {
                        identifier: Some("windjammer".to_string()),
                        inter_file_dependencies: true,
                        workspace_diagnostics: false,
                        ..Default::default()
                    },
                )),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "Windjammer Language Server".to_string(),
                version: Some("0.1.0".to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        tracing::info!("Windjammer LSP initialized");
        
        self.client
            .log_message(MessageType::INFO, "Windjammer Language Server initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        
        tracing::info!("File opened: {}", uri);
        
        let file_id = self.get_or_create_file_id(&uri);
        
        // Update the database with new source text
        // This will trigger incremental recompilation
        self.db.set_source_text(file_id, Arc::new(text));
        
        // Publish diagnostics
        publish_diagnostics(&self.client, &self.db, &uri, file_id).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        
        if let Some(change) = params.content_changes.into_iter().next() {
            let file_id = self.get_or_create_file_id(&uri);
            
            // Update source text - Salsa will automatically invalidate
            // dependent queries and recompute only what's needed
            self.db.set_source_text(file_id, Arc::new(change.text));
            
            // Publish updated diagnostics
            publish_diagnostics(&self.client, &self.db, &uri, file_id).await;
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        tracing::info!("File saved: {}", params.text_document.uri);
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        tracing::info!("File closed: {}", params.text_document.uri);
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        
        if let Some(file_id) = self.file_map.get(&uri).map(|e| *e) {
            let completions = get_completions(&self.db, file_id, position);
            Ok(Some(CompletionResponse::Array(completions)))
        } else {
            Ok(None)
        }
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        
        if let Some(file_id) = self.file_map.get(&uri).map(|e| *e) {
            Ok(get_hover_info(&self.db, file_id, position))
        } else {
            Ok(None)
        }
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        // TODO: Implement go-to-definition
        Ok(None)
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri;
        
        if let Some(file_id) = self.file_map.get(&uri).map(|e| *e) {
            let symbols = self.db.symbols(file_id);
            
            let document_symbols: Vec<DocumentSymbol> = symbols
                .iter()
                .map(|sym| DocumentSymbol {
                    name: sym.name.clone(),
                    detail: None,
                    kind: match sym.kind {
                        crate::database::SymbolKind::Function => SymbolKind::FUNCTION,
                        crate::database::SymbolKind::Struct => SymbolKind::STRUCT,
                        crate::database::SymbolKind::Enum => SymbolKind::ENUM,
                        crate::database::SymbolKind::Variable => SymbolKind::VARIABLE,
                        crate::database::SymbolKind::Parameter => SymbolKind::VARIABLE,
                        crate::database::SymbolKind::Field => SymbolKind::FIELD,
                    },
                    tags: None,
                    deprecated: None,
                    range: Range::new(
                        Position::new(sym.line, sym.col),
                        Position::new(sym.line, sym.col + sym.name.len() as u32),
                    ),
                    selection_range: Range::new(
                        Position::new(sym.line, sym.col),
                        Position::new(sym.line, sym.col + sym.name.len() as u32),
                    ),
                    children: None,
                })
                .collect();
            
            Ok(Some(DocumentSymbolResponse::Nested(document_symbols)))
        } else {
            Ok(None)
        }
    }
}


//! # Windjammer MCP Server
//!
//! Model Context Protocol (MCP) server for AI-powered Windjammer development.
//!
//! This crate provides a complete MCP server implementation that enables AI assistants
//! (Claude, ChatGPT, etc.) to understand, analyze, and generate Windjammer code.
//!
//! ## Features
//!
//! - **Code Understanding**: Parse, analyze, and explain Windjammer code
//! - **Code Generation**: Generate idiomatic Windjammer from natural language
//! - **Refactoring**: Extract functions, inline variables, rename symbols
//! - **Error Handling**: Explain errors and suggest fixes
//! - **Workspace Operations**: Search code, get context, list symbols
//! - **Shared Infrastructure**: Uses same Salsa database as LSP for consistency
//!
//! ## Example
//!
//! ```rust,no_run
//! use windjammer_mcp::McpServer;
//!
//! #[tokio::main]
//! async fn main() {
//!     let server = McpServer::new().await.unwrap();
//!     server.run_stdio().await.unwrap();
//! }
//! ```

pub mod error;
pub mod protocol;
pub mod server;
pub mod tools;

pub use error::{McpError, McpResult};
pub use server::McpServer;

/// MCP protocol version (2025-06-18 spec)
pub const MCP_VERSION: &str = "2025-06-18";

/// Windjammer MCP server version
pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

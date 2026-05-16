use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// Serialized visual graph (`windjammer-vgraph`).
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct VsDocument {
    pub format: String,
    pub version: u32,
    pub module_name: String,
    /// Optional human note stored by the editor (ignored by lowering).
    #[serde(default)]
    pub comment: Option<String>,
    pub nodes: Vec<VsNode>,
    pub edges: Vec<VsEdge>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct VsNode {
    pub id: String,
    #[serde(flatten)]
    pub kind: VsNodeKind,
}

/// Node payload: `kind` tag + optional parameters.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct VsNodeKind {
    pub kind: String,
    #[serde(default)]
    pub payload: HashMap<String, JsonValue>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EdgeKind {
    Exec,
    Data,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct VsEdge {
    pub kind: EdgeKind,
    pub from: VsEndpoint,
    pub to: VsEndpoint,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct VsEndpoint {
    pub node: String,
    pub pin: String,
}

//! Embedded agent index artifacts for MCP resources and tools.

use serde_json::Value;
use std::path::PathBuf;

fn agent_index_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../agent_index")
}

pub fn load_embedded_index() -> Result<Value, String> {
    let path = agent_index_dir().join("index_meta.json");
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

pub fn load_errors_json() -> Result<Value, String> {
    let path = agent_index_dir().join("errors.json");
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

pub fn resource_uri_list() -> Vec<(String, String)> {
    vec![
        (
            "windjammer://errors/catalog".to_string(),
            "Windjammer error code catalog".to_string(),
        ),
        (
            "windjammer://stdlib/overview".to_string(),
            "Windjammer stdlib overview".to_string(),
        ),
        (
            "windjammer://spec/tests".to_string(),
            "Language spec test index".to_string(),
        ),
    ]
}

pub fn read_resource(uri: &str) -> Result<String, String> {
    let base = agent_index_dir();
    if uri.starts_with("windjammer://errors/WJ") {
        let code = uri.trim_start_matches("windjammer://errors/");
        let catalog = load_errors_json()?;
        if let Some(entry) = catalog.get(code) {
            return serde_json::to_string_pretty(entry).map_err(|e| e.to_string());
        }
        return Err(format!("Unknown error code: {}", code));
    }
    let path = match uri {
        "windjammer://errors/catalog" => base.join("errors.json"),
        "windjammer://stdlib/overview" => base.join("stdlib.json"),
        "windjammer://spec/tests" => base.join("spec.json"),
        _ => return Err(format!("Unknown resource URI: {}", uri)),
    };
    std::fs::read_to_string(path).map_err(|e| e.to_string())
}

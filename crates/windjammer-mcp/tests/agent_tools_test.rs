//! The agent-loop tool surface: check / run / test / format / explain / docs-search, each with a JSON schema
//! and behaviour equal to the CLI. `run_code` and `test_code` spawn `wj`; set `WJ_BIN` or build it first
//! (`cargo build --bin wj`): the tests fail with instructions when it cannot be found.

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use windjammer_lsp::database::WindjammerDatabase;
use windjammer_mcp::protocol::ToolContent;
use windjammer_mcp::tools::ToolRegistry;

fn locate_wj() {
    if std::env::var("WJ_BIN").is_ok() {
        return;
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut candidates = vec![];
    if let Ok(t) = std::env::var("CARGO_TARGET_DIR") {
        candidates.push(PathBuf::from(&t).join("debug/wj"));
        candidates.push(PathBuf::from(&t).join("release/wj"));
    }
    candidates.push(manifest.join("../../target/debug/wj"));
    candidates.push(manifest.join("../../target/release/wj"));
    for c in candidates {
        if c.exists() {
            std::env::set_var("WJ_BIN", c);
            return;
        }
    }
    panic!("wj binary not found: run `cargo build --bin wj` or set WJ_BIN");
}

fn registry() -> ToolRegistry {
    locate_wj();
    ToolRegistry::new(Arc::new(Mutex::new(WindjammerDatabase::new())))
}

async fn call(name: &str, args: serde_json::Value) -> (bool, serde_json::Value) {
    let r = registry().call_tool(name, args).await.expect("tool call");
    let text = match &r.content[0] {
        ToolContent::Text { text } => text.clone(),
        _ => panic!("expected text content"),
    };
    (r.is_error, serde_json::from_str(&text).unwrap_or_else(|_| panic!("not JSON: {}", text)))
}

#[test]
fn agent_loop_tools_have_json_schemas() {
    let tools = tokio::runtime::Runtime::new().unwrap().block_on(async { registry().list_tools() });
    for (name, required) in [
        ("check_code", "code"),
        ("run_code", "code"),
        ("test_code", "code"),
        ("format_code", "code"),
        ("docs_search", "query"),
        ("explain_error", "error"),
    ] {
        let t = tools.iter().find(|t| t.name == name).unwrap_or_else(|| panic!("tool {} missing", name));
        assert!(!t.description.is_empty());
        let req = t.input_schema["required"].as_array().unwrap_or_else(|| panic!("{} schema has no required list", name));
        assert!(req.iter().any(|r| r == required), "{} must require `{}`", name, required);
        assert_eq!(t.input_schema["type"], "object");
    }
}

#[tokio::test]
async fn check_code_accepts_good_and_reports_lint_with_span() {
    let (err, v) = call("check_code", serde_json::json!({"code": "fn main() {\n    println(\"hi\")\n}\n"})).await;
    assert!(!err);
    assert_eq!(v["success"], true);

    let src = "fn main() {\n    let v = [1, 2]\n    let n = v.iter().count()\n    println(\"${n}\")\n}\n";
    let (err, v) = call("check_code", serde_json::json!({"code": src})).await;
    assert!(err, "errors must set is_error");
    assert_eq!(v["success"], false);
    assert_eq!(v["diagnostics"][0]["code"], "W0003");
    assert_eq!(v["diagnostics"][0]["span"]["start_line"], 3);
    assert_eq!(v["diagnostics"][0]["span"]["file"], "main.wj", "spans must not leak scratch paths");
}

#[tokio::test]
async fn check_code_reports_parse_errors() {
    let (err, v) = call("check_code", serde_json::json!({"code": "fn main() { let x = }"})).await;
    assert!(err);
    assert_eq!(v["diagnostics"][0]["code"], "parse");
}

#[tokio::test]
async fn run_code_captures_stdout_and_exit_codes() {
    let (err, v) = call("run_code", serde_json::json!({"code": "fn main() {\n    println(\"${6 * 7}\")\n}\n"})).await;
    assert!(!err);
    assert_eq!(v["stdout"], "42\n");
    assert_eq!(v["exit_code"], 0);

    let (err, v) = call("run_code", serde_json::json!({"code": "fn main() {\n    println(\"x\")\n    panic(\"boom\")\n}\n"})).await;
    assert!(err);
    assert_eq!(v["exit_code"], 101);
    assert_eq!(v["stdout"], "x\n");
    assert!(v["stderr"].as_str().unwrap().contains("boom"));
}

#[tokio::test]
async fn run_code_enforces_timeout() {
    let src = "fn main() {\n    let mut i = 0\n    loop { i += 1 }\n}\n";
    let (err, v) = call("run_code", serde_json::json!({"code": src, "timeout_secs": 1})).await;
    assert!(err);
    assert_eq!(v["timed_out"], true);
}

#[tokio::test]
async fn test_code_reports_pass_and_fail() {
    let pass = "@test\nfn ok() {\n    assert_eq(1 + 1, 2)\n}\n";
    let (err, v) = call("test_code", serde_json::json!({"code": pass})).await;
    assert!(!err, "{}", v);
    assert_eq!(v["passed"], true);
    assert!(v["summary"].as_str().unwrap().contains("1 passed; 0 failed"));

    let fail = "@test\nfn bad() {\n    assert_eq(1 + 1, 3)\n}\n";
    let (err, v) = call("test_code", serde_json::json!({"code": fail})).await;
    assert!(err);
    assert_eq!(v["passed"], false);
    assert!(v["summary"].as_str().unwrap().contains("1 failed"));
}

#[tokio::test]
async fn format_code_is_idempotent_and_reports_errors() {
    let messy = "fn main(){\nprintln(\"x\")\n}\n";
    let (_, v) = call("format_code", serde_json::json!({"code": messy})).await;
    let once = v["formatted"].as_str().unwrap().to_string();
    assert!(once.contains("fn main() {"));
    let (_, v2) = call("format_code", serde_json::json!({"code": once})).await;
    assert_eq!(v2["changed"], false, "formatting must be idempotent");
}

#[tokio::test]
async fn docs_search_finds_apis_and_ranks_exact_names_first() {
    let (_, v) = call("docs_search", serde_json::json!({"query": "sort_by_key", "limit": 3})).await;
    assert!(v["count"].as_u64().unwrap() >= 1);
    assert_eq!(v["results"][0]["name"], "sort_by_key");
    let (_, v) = call("docs_search", serde_json::json!({"query": "zzz-nothing"})).await;
    assert_eq!(v["count"], 0);
}

#[tokio::test]
async fn explain_error_covers_catalog_codes() {
    let (err, v) = call("explain_error", serde_json::json!({"error": "error WJ0001: cannot find value"})).await;
    assert!(!err);
    assert_eq!(v["success"], true);
    assert!(!v["explanation"].as_str().unwrap().is_empty());
}

#[tokio::test]
async fn invalid_arguments_are_rejected() {
    let r = registry().call_tool("check_code", serde_json::json!({"nope": 1})).await;
    assert!(r.is_err(), "missing `code` must be a validation error");
}

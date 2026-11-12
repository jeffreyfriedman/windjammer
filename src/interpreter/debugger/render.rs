//! Text rendering of debugger events, for people (`--json` prints the events themselves).

use serde_json::Value as Json;

fn at(e: &Json) -> String {
    match (e["file"].as_str(), e["line"].as_u64()) {
        (Some(f), Some(l)) => format!(" at {f}:{l}:{}", e["column"].as_u64().unwrap_or(1)),
        _ => String::new(),
    }
}

pub fn text(e: &Json) -> String {
    let s = |k: &str| e[k].as_str().unwrap_or("").to_string();
    match e["event"].as_str().unwrap_or("") {
        "stopped" => {
            let mut t = format!("[step {}] {} in {}{}", e["step"], s("reason"), s("function"), at(e));
            if let Some(w) = e["watch"].as_str() {
                let shown = |k: &str| e[k].as_str().map_or("<unset>".to_string(), String::from);
                t.push_str(&format!(": {w}: {} -> {}", shown("old"), shown("new")));
            }
            if let Some(c) = e["condition"].as_str() {
                t.push_str(&format!(": if {c}"));
            }
            if let Some(m) = e["message"].as_str() {
                t.push_str(&format!(": {m}"));
            }
            t
        }
        "returned" => format!("{} returned {}", s("function"), e["value"].as_str().unwrap_or("()")),
        "watching" => format!("watching {} in {} (now {})", s("name"), s("function"), e["value"].as_str().unwrap_or("<unset>")),
        "value" => format!("{} = {}", s("name"), s("value")),
        "locals" => {
            let b = e["bindings"].as_array().cloned().unwrap_or_default();
            let parts: Vec<String> = b.iter().map(|x| format!("{} = {}", x["name"].as_str().unwrap_or(""), x["value"].as_str().unwrap_or(""))).collect();
            format!("locals of {}: {}", s("function"), if parts.is_empty() { "(none)".into() } else { parts.join(", ") })
        }
        "stack" => {
            let f = e["frames"].as_array().cloned().unwrap_or_default();
            f.iter().enumerate().map(|(i, x)| format!("#{i} {}{}", x["function"].as_str().unwrap_or(""), at(x))).collect::<Vec<_>>().join("\n")
        }
        "output" => s("text").trim_end_matches('\n').to_string(),
        "exited" => {
            let mut t = format!("[exited] status {} after {} steps", e["code"], e["steps"]);
            if let Some(m) = e["message"].as_str() {
                t.push_str(&format!("\n{}", m.trim_end()));
            }
            t
        }
        "error" => format!("error: {}: {}", s("command"), s("message")),
        other => format!("{other}: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn events_render_as_one_readable_line() {
        let e = json!({"event": "stopped", "reason": "failure", "step": 7, "function": "main", "file": "main.wj", "line": 4, "column": 5, "message": "boom"});
        assert_eq!(super::text(&e), "[step 7] failure in main at main.wj:4:5: boom");
        assert_eq!(super::text(&json!({"event": "value", "name": "x", "value": "3"})), "x = 3");
        assert_eq!(super::text(&json!({"event": "exited", "code": 101, "steps": 9})), "[exited] status 101 after 9 steps");
    }
}

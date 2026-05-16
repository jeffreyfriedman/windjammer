//! Deterministic lowering: v1 supports `event_on_start`, `literal_float`, `add_float`, and `multiply_float`.

use super::schema::{EdgeKind, VsDocument, VsEdge, VsNode, VsNodeKind};
use serde_json::Value as JsonValue;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;

#[derive(Debug, PartialEq)]
pub enum LowerError {
    Format { expected: &'static str, got: String },
    Version(u32),
    UnknownNodeKind(String),
    MissingPayload { node: String, key: String },
    MissingDataSource { node: String, pin: String },
    MultipleExecPredecessors { node: String },
}

impl fmt::Display for LowerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LowerError::Format { expected, got } => {
                write!(f, "expected format {expected}, got {got}")
            }
            LowerError::Version(v) => write!(f, "unsupported vgraph version {v}"),
            LowerError::UnknownNodeKind(k) => write!(f, "unknown node kind {k}"),
            LowerError::MissingPayload { node, key } => {
                write!(f, "node {node} missing payload key {}", key.as_str())
            }
            LowerError::MissingDataSource { node, pin } => {
                write!(f, "node {node} has no data source for pin {pin}")
            }
            LowerError::MultipleExecPredecessors { node } => {
                write!(
                    f,
                    "node {node} has more than one exec predecessor (use sequence in future)"
                )
            }
        }
    }
}

impl std::error::Error for LowerError {}

pub fn lower_document_to_windjammer(doc: &VsDocument) -> Result<String, LowerError> {
    if doc.format != "windjammer-vgraph" {
        return Err(LowerError::Format {
            expected: "windjammer-vgraph",
            got: doc.format.clone(),
        });
    }
    if doc.version != 1 {
        return Err(LowerError::Version(doc.version));
    }

    let nodes: HashMap<String, &VsNode> = doc.nodes.iter().map(|n| (n.id.clone(), n)).collect();

    let mut roots = Vec::new();
    let mut exec_children: HashMap<String, Vec<String>> = HashMap::new();
    let mut exec_pred_count: HashMap<String, usize> = HashMap::new();

    for e in &doc.edges {
        if e.kind == EdgeKind::Exec {
            exec_children
                .entry(e.from.node.clone())
                .or_default()
                .push(e.to.node.clone());
            *exec_pred_count.entry(e.to.node.clone()).or_insert(0) += 1;
        }
    }

    for n in &doc.nodes {
        if n.kind.kind == "event_on_start" {
            roots.push(n.id.clone());
        }
    }

    if roots.is_empty() {
        // Still emit empty module skeleton for authoring-time saves.
        return Ok(emit_module(&doc.module_name, &["// (no event_on_start entry)"]));
    }

    let mut lines: Vec<String> = Vec::new();
    let mut emitted: HashSet<String> = HashSet::new();

    for root in roots {
        let mut visited = HashSet::new();
        let order = exec_walk(&root, &exec_children, &mut visited);

        for nid in order {
            let node = nodes
                .get(&nid)
                .ok_or_else(|| LowerError::UnknownNodeKind(format!("missing node {nid}")))?;
            if *exec_pred_count.get(&nid).unwrap_or(&0) > 1 {
                return Err(LowerError::MultipleExecPredecessors { node: nid.clone() });
            }

            match node.kind.kind.as_str() {
                "event_on_start" => {
                    lines.push(format!(
                        "    // @vs entry event_on_start id={}",
                        sanitize_ident(&nid)
                    ));
                }
                "literal_float" => {
                    // Pure data fan-in: emit when reached on an exec edge, else wait for consumers.
                    emit_literal_if_needed(doc, &nodes, &mut emitted, &mut lines, &nid)?;
                }
                "add_float" | "multiply_float" => {
                    emit_value_producer(doc, &nodes, &mut emitted, &mut lines, &nid)?;
                }
                other => return Err(LowerError::UnknownNodeKind(other.into())),
            }
        }
    }

    let fn_lines = assemble_function(&doc.module_name, &lines);
    Ok(fn_lines.join("\n"))
}

fn assemble_function(module_name: &str, body: &[String]) -> Vec<String> {
    let fname = sanitize_ident(module_name);
    let mut out = vec![
        format!("// Generated from windjammer-vgraph — edit graph, not hand-merge."),
        format!("pub fn {}_on_start() {{", fname),
    ];
    for line in body {
        out.push(line.clone());
    }
    out.push("}".into());
    out
}

fn emit_module(module_name: &str, body: &[&str]) -> String {
    let fname = sanitize_ident(module_name);
    let mut lines = vec![
        format!("// Generated from windjammer-vgraph ({module_name})"),
        format!("pub fn {fname}_on_start() {{"),
    ];
    for b in body {
        lines.push(format!("    {b}"));
    }
    lines.push("}".into());
    lines.join("\n")
}

fn exec_walk(
    root: &str,
    children: &HashMap<String, Vec<String>>,
    visited: &mut HashSet<String>,
) -> VecDeque<String> {
    let mut out = VecDeque::new();
    let mut queue = VecDeque::new();
    queue.push_back(root.to_string());

    while let Some(n) = queue.pop_front() {
        if visited.contains(&n) {
            continue;
        }
        visited.insert(n.clone());
        out.push_back(n.clone());

        if let Some(kids) = children.get(&n) {
            let mut sorted = kids.clone();
            sorted.sort();
            for c in sorted {
                queue.push_back(c);
            }
        }
    }
    out.into()
}

fn payload_f64(kind: &VsNodeKind, node_id: &str, key: &str) -> Result<f64, LowerError> {
    let v = kind.payload.get(key).ok_or_else(|| LowerError::MissingPayload {
        node: node_id.into(),
        key: key.into(),
    })?;
    match v {
        JsonValue::Number(n) => n.as_f64().ok_or_else(|| LowerError::MissingPayload {
            node: node_id.into(),
            key: key.into(),
        }),
        _ => Err(LowerError::MissingPayload {
            node: node_id.into(),
            key: key.into(),
        }),
    }
}

fn resolve_data_expr(
    edges: &[VsEdge],
    nodes: &HashMap<String, &VsNode>,
    target_node: &str,
    target_pin: &str,
) -> Result<String, LowerError> {
    let mut src = None;
    for e in edges {
        if e.kind == EdgeKind::Data
            && e.to.node == target_node
            && e.to.pin == target_pin
        {
            src = Some(e.from.clone());
            break;
        }
    }
    let from = src.ok_or_else(|| LowerError::MissingDataSource {
        node: target_node.into(),
        pin: target_pin.into(),
    })?;

    let n = nodes
        .get(&from.node)
        .ok_or_else(|| LowerError::UnknownNodeKind(from.node.clone()))?;

    match n.kind.kind.as_str() {
        "literal_float" => {
            let v = payload_f64(&n.kind, &n.id, "value")?;
            Ok(format_float(v))
        }
        "add_float" => Ok(data_sym(&from.node, "out")),
        "multiply_float" => Ok(data_sym(&from.node, "out")),
        _ => Err(LowerError::UnknownNodeKind(format!(
            "unsupported data producer {} / {}",
            n.kind.kind, n.id
        ))),
    }
}

/// Node ids whose values must be emitted before `target` can read `pin`.
fn data_producers_for_pin(
    doc: &VsDocument,
    target: &str,
    pin: &str,
) -> Result<Vec<String>, LowerError> {
    let mut src = None;
    for e in &doc.edges {
        if e.kind == EdgeKind::Data && e.to.node == target && e.to.pin == pin {
            src = Some(e.from.node.clone());
            break;
        }
    }
    let id = src.ok_or_else(|| LowerError::MissingDataSource {
        node: target.into(),
        pin: pin.into(),
    })?;
    Ok(vec![id])
}

fn emit_literal_if_needed(
    doc: &VsDocument,
    nodes: &HashMap<String, &VsNode>,
    emitted: &mut HashSet<String>,
    lines: &mut Vec<String>,
    nid: &str,
) -> Result<(), LowerError> {
    if emitted.contains(nid) {
        return Ok(());
    }
    emit_value_producer(doc, nodes, emitted, lines, nid)
}

fn emit_value_producer(
    doc: &VsDocument,
    nodes: &HashMap<String, &VsNode>,
    emitted: &mut HashSet<String>,
    lines: &mut Vec<String>,
    nid: &str,
) -> Result<(), LowerError> {
    if emitted.contains(nid) {
        return Ok(());
    }
    let node = nodes
        .get(nid)
        .ok_or_else(|| LowerError::UnknownNodeKind(format!("missing node {nid}")))?;
    match node.kind.kind.as_str() {
        "literal_float" => {
            let v = payload_f64(&node.kind, nid, "value")?;
            let sym = data_sym(nid, "value");
            lines.push(format!(
                "    let {} = {} // @vs literal_float id={}",
                sym,
                format_float(v),
                sanitize_ident(nid)
            ));
            emitted.insert(nid.into());
            Ok(())
        }
        "add_float" => {
            let pa = data_producers_for_pin(doc, nid, "a")?;
            for p in pa {
                emit_value_producer(doc, nodes, emitted, lines, &p)?;
            }
            let pb = data_producers_for_pin(doc, nid, "b")?;
            for p in pb {
                emit_value_producer(doc, nodes, emitted, lines, &p)?;
            }
            let a = resolve_data_expr(&doc.edges, nodes, nid, "a")?;
            let b = resolve_data_expr(&doc.edges, nodes, nid, "b")?;
            let sym = data_sym(nid, "out");
            lines.push(format!(
                "    let {} = {} + {} // @vs add_float id={}",
                sym,
                a,
                b,
                sanitize_ident(nid)
            ));
            emitted.insert(nid.into());
            Ok(())
        }
        "multiply_float" => {
            let pa = data_producers_for_pin(doc, nid, "a")?;
            for p in pa {
                emit_value_producer(doc, nodes, emitted, lines, &p)?;
            }
            let pb = data_producers_for_pin(doc, nid, "b")?;
            for p in pb {
                emit_value_producer(doc, nodes, emitted, lines, &p)?;
            }
            let a = resolve_data_expr(&doc.edges, nodes, nid, "a")?;
            let b = resolve_data_expr(&doc.edges, nodes, nid, "b")?;
            let sym = data_sym(nid, "out");
            lines.push(format!(
                "    let {} = {} * {} // @vs multiply_float id={}",
                sym,
                a,
                b,
                sanitize_ident(nid)
            ));
            emitted.insert(nid.into());
            Ok(())
        }
        other => Err(LowerError::UnknownNodeKind(other.into())),
    }
}

fn data_sym(node_id: &str, pin: &str) -> String {
    format!("vs_{}_{}", sanitize_ident(node_id), sanitize_ident(pin))
}

fn sanitize_ident(s: &str) -> String {
    let mut out = String::new();
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
        } else if ch == '_' {
            out.push('_');
        } else {
            out.push('_');
            let code = (ch as u32).min(0xffff);
            out.push_str(&format!("{code:x}"));
        }
    }
    if out.is_empty() {
        return "empty_id".into();
    }
    if out.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        format!("n{out}")
    } else {
        out
    }
}

fn format_float(v: f64) -> String {
    if v.is_finite() {
        format!("{v:.6}")
    } else {
        "0.0".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::visual_script::schema::{VsDocument, VsEdge, VsEndpoint, VsNode, VsNodeKind};
    use serde_json::json;
    use std::collections::HashMap;

    fn sample_doc() -> VsDocument {
        VsDocument {
            format: "windjammer-vgraph".into(),
            version: 1,
            module_name: "demo_add".into(),
            comment: None,
            nodes: vec![
                VsNode {
                    id: "e1".into(),
                    kind: VsNodeKind {
                        kind: "event_on_start".into(),
                        payload: HashMap::new(),
                    },
                },
                VsNode {
                    id: "a".into(),
                    kind: VsNodeKind {
                        kind: "literal_float".into(),
                        payload: HashMap::from([("value".into(), json!(2.0))]),
                    },
                },
                VsNode {
                    id: "b".into(),
                    kind: VsNodeKind {
                        kind: "literal_float".into(),
                        payload: HashMap::from([("value".into(), json!(3.0))]),
                    },
                },
                VsNode {
                    id: "sum".into(),
                    kind: VsNodeKind {
                        kind: "add_float".into(),
                        payload: HashMap::new(),
                    },
                },
            ],
            edges: vec![
                VsEdge {
                    kind: EdgeKind::Exec,
                    from: VsEndpoint {
                        node: "e1".into(),
                        pin: "exec_out".into(),
                    },
                    to: VsEndpoint {
                        node: "sum".into(),
                        pin: "exec_in".into(),
                    },
                },
                VsEdge {
                    kind: EdgeKind::Data,
                    from: VsEndpoint {
                        node: "a".into(),
                        pin: "value".into(),
                    },
                    to: VsEndpoint {
                        node: "sum".into(),
                        pin: "a".into(),
                    },
                },
                VsEdge {
                    kind: EdgeKind::Data,
                    from: VsEndpoint {
                        node: "b".into(),
                        pin: "value".into(),
                    },
                    to: VsEndpoint {
                        node: "sum".into(),
                        pin: "b".into(),
                    },
                },
            ],
        }
    }

    #[test]
    fn lowers_add_chain() {
        let wj = lower_document_to_windjammer(&sample_doc()).expect("lower");
        assert!(wj.contains("vs_sum_out"));
        assert!(wj.contains("2.000000"));
        assert!(wj.contains("3.000000"));
        assert!(wj.contains("pub fn demo_add_on_start"));
    }
}

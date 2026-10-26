// consumes: tests/effects/vectors.json, crates/windjammer-runtime/src/caps_core, std/runtime/js/wj_std.mjs, std/runtime/go/wj_std.go
//! Conformance vectors for the capability model (ADR-0021): one table, every implementation.
//!
//! * Rust: the shared model (`caps_core`, used by the compiler, the interpreter and compiled programs);
//! * JavaScript: `std/runtime/js/wj_std.mjs` (run with node, when installed);
//! * Go: `std/runtime/go/wj_std.go` (run with go, when installed).
//! A port that disagrees on any case fails here, so Go and JS cannot drift from the Rust model.

use std::path::{Path, PathBuf};
use std::process::Command;
use windjammer::ir::effect_facts::{EffectKind, Grant};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn vectors() -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(root().join("tests/effects/vectors.json")).unwrap()).unwrap()
}

fn grant_of(case: &serde_json::Value, proj: &str) -> Grant {
    let mut g = Grant::sandbox(Path::new(proj));
    for (name, pats) in case["allow"].as_object().unwrap() {
        let kind = EffectKind::parse_builtin(name).unwrap_or_else(|| EffectKind::User(name.clone()));
        let pats = pats.as_array().unwrap();
        if pats.is_empty() {
            g.declare_empty(kind.clone());
        }
        for p in pats {
            g.allow(kind.clone(), p.as_str().unwrap());
        }
    }
    for d in case["deny"].as_array().unwrap() {
        g.deny(EffectKind::parse_builtin(d.as_str().unwrap()).unwrap());
    }
    g
}

#[test]
fn rust_model_agrees_with_the_vectors() {
    let v = vectors();
    let proj = v["root"].as_str().unwrap();
    let mut failures = Vec::new();
    let cases = v["cases"].as_array().unwrap();
    for c in cases {
        let g = grant_of(c, proj);
        let kind = EffectKind::parse_builtin(c["kind"].as_str().unwrap()).unwrap();
        let got = if g.check(&kind, c["resource"].as_str()).is_ok() { "allow" } else { "deny" };
        if got != c["expect"].as_str().unwrap() {
            failures.push(format!("{}: want {}, got {got}", c["name"], c["expect"]));
        }
    }
    assert!(cases.len() >= 60, "{} vectors", cases.len());
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

fn have(tool: &str) -> bool {
    Command::new(tool).arg(if tool == "go" { "version" } else { "--version" }).output().map(|o| o.status.success()).unwrap_or(false)
}

#[test]
fn javascript_runtime_agrees_with_the_vectors() {
    if !have("node") {
        eprintln!("node not installed: skipping the JavaScript vectors");
        return;
    }
    let dir = std::env::temp_dir().join(format!("wj_vec_js_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for f in ["wj_core.mjs", "wj_std.mjs"] {
        std::fs::copy(root().join("std/runtime/js").join(f), dir.join(f)).unwrap();
    }
    std::fs::write(
        dir.join("run.mjs"),
        r#"import { readFileSync } from "node:fs";
import { $caps_vector } from "./wj_std.mjs";
const v = JSON.parse(readFileSync(process.argv[2], "utf8"));
const out = [];
for (const c of v.cases) {
  if (!c.portable) continue;
  const spec = { root: v.root, allow: Object.fromEntries(Object.entries(c.allow).map(([k, p]) => [k, p])), deny: c.deny };
  const got = $caps_vector(spec, c.kind, c.resource === null ? undefined : c.resource);
  if (got !== c.expect) out.push(`${c.name}: want ${c.expect}, got ${got}`);
}
console.log(out.join("\n"));
"#,
    )
    .unwrap();
    let out = Command::new("node").arg(dir.join("run.mjs")).arg(root().join("tests/effects/vectors.json")).output().unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    assert!(out.status.success(), "node failed: {}", String::from_utf8_lossy(&out.stderr));
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.trim().is_empty(), "the JavaScript runtime disagrees with the Rust model:\n{text}");
}

#[test]
fn go_runtime_agrees_with_the_vectors() {
    if !have("go") {
        eprintln!("go not installed: skipping the Go vectors");
        return;
    }
    let dir = std::env::temp_dir().join(format!("wj_vec_go_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for f in ["wj_core.go", "wj_std.go"] {
        std::fs::copy(root().join("std/runtime/go").join(f), dir.join(f)).unwrap();
    }
    std::fs::write(dir.join("go.mod"), "module vectors\n\ngo 1.22\n").unwrap();
    std::fs::write(
        dir.join("main.go"),
        r#"package main

import (
	"encoding/json"
	"fmt"
	"os"
)

type vcase struct {
	Name     string              `json:"name"`
	Allow    map[string][]string `json:"allow"`
	Deny     []string            `json:"deny"`
	Kind     string              `json:"kind"`
	Resource *string             `json:"resource"`
	Expect   string              `json:"expect"`
	Portable bool                `json:"portable"`
}

func main() {
	data, _ := os.ReadFile(os.Args[1])
	var v struct {
		Root  string  `json:"root"`
		Cases []vcase `json:"cases"`
	}
	if err := json.Unmarshal(data, &v); err != nil {
		fmt.Println("bad json:", err)
		os.Exit(2)
	}
	for _, c := range v.Cases {
		if !c.Portable {
			continue
		}
		got := wjstd_caps_vector(wjCapSpec{Root: v.Root, Allow: c.Allow, Deny: c.Deny}, c.Kind, c.Resource)
		if got != c.Expect {
			fmt.Printf("%s: want %s, got %s\n", c.Name, c.Expect, got)
		}
	}
}
"#,
    )
    .unwrap();
    let out = Command::new("go").arg("run").arg(".").arg(root().join("tests/effects/vectors.json")).current_dir(&dir).output().unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    assert!(out.status.success(), "go failed: {}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.trim().is_empty(), "the Go runtime disagrees with the Rust model:\n{text}");
}

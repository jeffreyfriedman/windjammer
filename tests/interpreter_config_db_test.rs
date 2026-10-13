#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "interpreter_tests",
))]

//! `std::config` and `std::db` in the interpreter.
//!
//! config: the expected output was produced by `crates/windjammer-runtime/src/config.rs` itself,
//! compiled standalone with rustc/cargo (a `#[path]` include driving the same calls), so the
//! interpreter must agree with the runtime byte for byte.
//! db: the oracle is Python's `sqlite3` replaying the same statements; cells are rendered with the
//! runtime's rule (TEXT and INTEGER as strings, anything else as ""), errors in sqlx's format.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

fn run(src: &str) -> (String, String, i32) {
    let dir = TempDir::new().unwrap();
    let f = dir.path().join("prog.wj");
    fs::write(&f, src).unwrap();
    let o = Command::new(WJ).args(["run", "--interpret"]).arg(&f).current_dir(dir.path()).output().unwrap();
    (String::from_utf8_lossy(&o.stdout).to_string(), String::from_utf8_lossy(&o.stderr).to_string(), o.status.code().unwrap_or(-1))
}

const CONFIG: &str = r##"use std::config
use std::collections::HashMap

fn main() {
    let text = "HOST = \"localhost\"\n[server]\nPORT = 3000\nTAGS = [\"a\", \"b\"]\nDEBUG = true\n"
    match config::to_json(text, "toml") {
        Ok(j) => println(j),
        Err(e) => println("ERR ${e}"),
    }
    let mut defaults = HashMap::new()
    defaults.insert("HOST", "0.0.0.0")
    defaults.insert("server.PORT", "8080")
    defaults.insert("LOG", "info")
    let mut env = HashMap::new()
    env.insert("LOG", "debug")
    env.insert("IGNORED", "x")
    match config::resolve_text(defaults, text, "toml", env) {
        Ok(m) => {
            let mut keys: Vec<string> = m.keys().cloned().collect()
            keys.sort()
            for k in keys {
                println("${k}=${m[&k]}")
            }
        }
        Err(e) => println("ERR ${e}"),
    }
    match config::parse_flat("a:\n  b: 1\n  c: [1, 2]\n  d: ~\n", "yml") {
        Ok(m) => {
            let mut keys: Vec<string> = m.keys().cloned().collect()
            keys.sort()
            for k in keys {
                println("${k}=[${m[&k]}]")
            }
        }
        Err(e) => println("ERR ${e}"),
    }
    match config::parse_flat("  ", "toml") {
        Ok(_) => println("unexpected"),
        Err(e) => println("ERR ${e}"),
    }
    match config::to_json("a = 1", "ini") {
        Ok(_) => println("unexpected"),
        Err(e) => println("ERR ${e}"),
    }
}
"##;

/// Output of the runtime's `config.rs` for the same calls.
const CONFIG_RUNTIME_OUTPUT: &str = r##"{"HOST":"localhost","server":{"DEBUG":true,"PORT":3000,"TAGS":["a","b"]}}
HOST=localhost
LOG=debug
server.DEBUG=true
server.PORT=3000
server.TAGS=["a","b"]
a.b=[1]
a.c=[[1,2]]
a.d=[]
ERR empty toml
ERR unknown config format: ini
"##;

#[test]
fn config_matches_the_runtime() {
    let (out, err, code) = run(CONFIG);
    assert_eq!(code, 0, "{err}");
    assert_eq!(out, CONFIG_RUNTIME_OUTPUT);
}

#[test]
fn legacy_config_hooks_are_loud() {
    let (_, err, code) = run("use std::config\nfn main() {\n    println(config::get_or(\"k\", \"d\"))\n}\n");
    assert_ne!(code, 0);
    assert!(err.contains("config::get_or") && err.contains("no implementation in any backend"), "{err}");
}

const DB: &str = r##"use std::db

fn main() {
    match db::connect(":memory:") {
        Ok(conn) => {
            match conn.execute("CREATE TABLE items (id INTEGER, name TEXT, price REAL)", Vec::new()) {
                Ok(n) => println("create ${n}"),
                Err(e) => println("ERR ${e}"),
            }
            match conn.execute("INSERT INTO items (id, name, price) VALUES (?, ?, ?), (?, ?, ?)", vec!["1", "Alice", "1.5", "2", "Bob", "2"]) {
                Ok(n) => println("insert ${n}"),
                Err(e) => println("ERR ${e}"),
            }
            match conn.query("SELECT id, name, price FROM items WHERE id >= ? ORDER BY id", vec!["1"]) {
                Ok(rows) => {
                    for row in rows {
                        let id = row.get_int("id").unwrap_or(-1)
                        let name = row.get_string("name").unwrap_or("?")
                        let price = row.get_string_at(2).unwrap_or("?")
                        println("${id} ${name} [${price}]")
                    }
                }
                Err(e) => println("ERR ${e}"),
            }
            match conn.query("SELECT nope FROM items", Vec::new()) {
                Ok(_) => println("unexpected"),
                Err(e) => println("ERR ${e}"),
            }
            match conn.query("SELECT name FROM items", Vec::new()) {
                Ok(rows) => match rows[0].get_int("name") {
                    Ok(_) => println("unexpected"),
                    Err(e) => println("ERR ${e}"),
                },
                Err(e) => println("ERR ${e}"),
            }
        }
        Err(e) => println("ERR ${e}"),
    }
    match db::connect("/nonexistent/dir/x.db") {
        Ok(_) => println("unexpected"),
        Err(e) => println("ERR ${e}"),
    }
}
"##;

const DB_ORACLE: &str = r#"
import sqlite3
c = sqlite3.connect(":memory:")
def cell(v):
    return str(v) if isinstance(v, (int, str)) and not isinstance(v, bool) else ""
cur = c.execute("CREATE TABLE items (id INTEGER, name TEXT, price REAL)")
print("create", 0)
cur = c.execute("INSERT INTO items (id, name, price) VALUES (?, ?, ?), (?, ?, ?)", ["1", "Alice", "1.5", "2", "Bob", "2"])
print("insert", cur.rowcount)
for r in c.execute("SELECT id, name, price FROM items WHERE id >= ? ORDER BY id", ["1"]):
    print(f"{cell(r[0])} {cell(r[1])} [{cell(r[2])}]")
try:
    c.execute("SELECT nope FROM items")
except sqlite3.OperationalError as e:
    print(f"ERR error returned from database: (code: {e.sqlite_errorcode}) {e}")
name = c.execute("SELECT name FROM items").fetchone()[0]
try:
    int(name)
except ValueError:
    print("ERR column name is not int: invalid digit found in string")
try:
    sqlite3.connect("file:/nonexistent/dir/x.db?mode=rw", uri=True)
except sqlite3.OperationalError as e:
    print(f"ERR error returned from database: (code: {e.sqlite_errorcode}) {e}")
"#;

#[test]
fn db_sqlite_matches_python_sqlite3() {
    let Ok(o) = Command::new("python3").arg("-I").arg("-c").arg(DB_ORACLE).output() else { return };
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let (out, err, code) = run(DB);
    assert_eq!(code, 0, "{err}");
    assert_eq!(out, String::from_utf8_lossy(&o.stdout));
}

#[test]
fn db_postgres_and_csv_builders_are_loud() {
    let (out, _, _) = run("use std::db\nfn main() {\n    match db::connect(\"postgres://localhost/x\") {\n        Ok(_) => println(\"ok\"),\n        Err(e) => println(e),\n    }\n}\n");
    assert!(out.starts_with("postgres is not available in the interpreter"), "{out}");
    let (_, err, code) = run("use std::csv\nfn main() {\n    let r = csv::reader_with_delimiter(\"a;b\", 59)\n}\n");
    assert_ne!(code, 0);
    assert!(err.contains("csv::reader_with_delimiter") && err.contains("csv::parse"), "{err}");
}

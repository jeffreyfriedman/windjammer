//! `std::db` over SQLite (the library sqlx bundles for `windjammer_runtime::db`), with the
//! runtime's contract: `connect(":memory:" | path | "sqlite:..")`, parameters bound as TEXT in
//! order, cells read back as TEXT / INTEGER strings (anything else, NULL included, is ""), and
//! `execute` returning the affected-row count. A file must exist unless the URL asks for
//! `?mode=rwc` (sqlx's default). Postgres URLs are refused with an error: the interpreter has no
//! Postgres client (`wj run` does).

use super::*;
use crate::interpreter::native::{downcast, Native};
use libsqlite3_sys as ffi;
use std::any::Any;
use std::ffi::{CStr, CString};

struct Conn(*mut ffi::sqlite3);

impl Drop for Conn {
    fn drop(&mut self) {
        // SAFETY: the handle came from a successful sqlite3_open_v2 and is closed exactly once.
        unsafe { ffi::sqlite3_close(self.0) };
    }
}

impl Native for Conn {
    fn type_name(&self) -> &'static str {
        "Connection"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

struct Row(Vec<(String, String)>);

impl Native for Row {
    fn type_name(&self) -> &'static str {
        "Row"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// sqlx's rendering of a database error.
fn db_error(db: *mut ffi::sqlite3) -> String {
    // SAFETY: `db` is a live handle (possibly in an error state, which errmsg reports).
    unsafe {
        let code = ffi::sqlite3_extended_errcode(db);
        let msg = CStr::from_ptr(ffi::sqlite3_errmsg(db)).to_string_lossy();
        format!("error returned from database: (code: {code}) {msg}")
    }
}

fn open(url: &str) -> Result<Conn, String> {
    if url.starts_with("postgres://") || url.starts_with("postgresql://") {
        return Err(format!("postgres is not available in the interpreter: {url} (use `wj run`; sqlite paths and :memory: work)"));
    }
    let rest = url.strip_prefix("sqlite://").or_else(|| url.strip_prefix("sqlite:")).unwrap_or(url);
    let (path, query) = rest.split_once('?').unwrap_or((rest, ""));
    let mode = query.split('&').find_map(|kv| kv.strip_prefix("mode=")).unwrap_or("rw");
    let memory = path == ":memory:" || mode == "memory";
    let mut flags = match mode {
        "ro" => ffi::SQLITE_OPEN_READONLY,
        "rwc" => ffi::SQLITE_OPEN_READWRITE | ffi::SQLITE_OPEN_CREATE,
        _ => ffi::SQLITE_OPEN_READWRITE,
    };
    if memory {
        flags = ffi::SQLITE_OPEN_READWRITE | ffi::SQLITE_OPEN_CREATE | ffi::SQLITE_OPEN_MEMORY;
    }
    let c = CString::new(if memory { ":memory:" } else { path }).map_err(|e| e.to_string())?;
    let mut db = std::ptr::null_mut();
    // SAFETY: valid NUL-terminated path, out-pointer to a local; on failure the handle is closed.
    let rc = unsafe { ffi::sqlite3_open_v2(c.as_ptr(), &mut db, flags, std::ptr::null()) };
    let conn = Conn(db);
    if rc != ffi::SQLITE_OK {
        return Err(db_error(db));
    }
    Ok(conn)
}

/// Run every statement of `sql`, binding `params` in order across them; rows of all statements.
fn run(conn: &Conn, sql: &str, params: &[String]) -> Result<(Vec<Row>, i64), String> {
    let db = conn.0;
    let bytes = sql.as_bytes();
    let (mut rows, mut changes, mut offset, mut next_param) = (Vec::new(), 0i64, 0usize, 0usize);
    while offset < bytes.len() {
        let mut stmt = std::ptr::null_mut();
        let mut tail = std::ptr::null();
        let start = bytes[offset..].as_ptr() as *const std::os::raw::c_char;
        // SAFETY: `start..len` is inside `sql`; `tail` points back into the same buffer.
        let rc = unsafe { ffi::sqlite3_prepare_v2(db, start, (bytes.len() - offset) as i32, &mut stmt, &mut tail) };
        if rc != ffi::SQLITE_OK {
            return Err(db_error(db));
        }
        offset = tail as usize - bytes.as_ptr() as usize;
        if stmt.is_null() {
            continue; // whitespace or a comment
        }
        let r = step_all(db, stmt, params, &mut next_param, &mut rows);
        // SAFETY: finalize the statement prepared above exactly once.
        unsafe { ffi::sqlite3_finalize(stmt) };
        r?;
        // SAFETY: live handle.
        changes += unsafe { ffi::sqlite3_changes(db) } as i64;
    }
    Ok((rows, changes))
}

fn step_all(db: *mut ffi::sqlite3, stmt: *mut ffi::sqlite3_stmt, params: &[String], next: &mut usize, rows: &mut Vec<Row>) -> Result<(), String> {
    // SAFETY: `stmt` is a live prepared statement of `db`; text is copied (SQLITE_TRANSIENT).
    unsafe {
        for i in 1..=ffi::sqlite3_bind_parameter_count(stmt) {
            if let Some(p) = params.get(*next) {
                ffi::sqlite3_bind_text(stmt, i, p.as_ptr() as *const _, p.len() as i32, ffi::SQLITE_TRANSIENT());
            }
            *next += 1;
        }
        loop {
            match ffi::sqlite3_step(stmt) {
                ffi::SQLITE_ROW => {
                    let n = ffi::sqlite3_column_count(stmt);
                    let mut cols = Vec::with_capacity(n as usize);
                    for c in 0..n {
                        let name = CStr::from_ptr(ffi::sqlite3_column_name(stmt, c)).to_string_lossy().to_string();
                        let value = match ffi::sqlite3_column_type(stmt, c) {
                            ffi::SQLITE_INTEGER => ffi::sqlite3_column_int64(stmt, c).to_string(),
                            ffi::SQLITE_TEXT => {
                                let p = ffi::sqlite3_column_text(stmt, c);
                                let len = ffi::sqlite3_column_bytes(stmt, c) as usize;
                                String::from_utf8_lossy(std::slice::from_raw_parts(p, len)).to_string()
                            }
                            _ => String::new(),
                        };
                        cols.push((name, value));
                    }
                    rows.push(Row(cols));
                }
                ffi::SQLITE_DONE => return Ok(()),
                _ => return Err(db_error(db)),
            }
        }
    }
}

fn conn(v: &Value) -> R<&Conn> {
    downcast::<Conn>(v).ok_or_else(|| semantic("expected a db Connection"))
}

fn row(v: &Value) -> R<&Row> {
    downcast::<Row>(v).ok_or_else(|| semantic("expected a db Row"))
}

fn params(it: &mut Interpreter, a: &[Value]) -> R<Vec<String>> {
    let items = match a.get(1) {
        Some(_) => it.to_items(arg(a, 1, "params")?)?,
        None => Vec::new(),
    };
    items.into_iter().map(|v| it.display_string(&crate::interpreter::place::deref_value(v)?)).collect()
}

fn res(r: Result<Value, String>) -> Value {
    match r {
        Ok(v) => Value::ok(v),
        Err(e) => Value::err(Value::str(e)),
    }
}

pub fn register(r: &mut Registry) {
    r.type_name("Connection");
    r.type_name("Row");
    r.func("db::connect", |_, a| Ok(res(open(&arg_str(&a, 0, "connect")?).map(|c| Value::Native(Rc::new(c))))));
    r.native("Connection", "query", |it, v, a| {
        let (sql, ps) = (arg_str(&a, 0, "query")?, params(it, &a)?);
        Ok(res(run(conn(v)?, &sql, &ps).map(|(rows, _)| Value::vec(rows.into_iter().map(|r| Value::Native(Rc::new(r))).collect()))))
    });
    r.native("Connection", "execute", |it, v, a| {
        let (sql, ps) = (arg_str(&a, 0, "execute")?, params(it, &a)?);
        Ok(res(run(conn(v)?, &sql, &ps).map(|(_, n)| Value::Int(n as i128, IntTy::I64))))
    });
    r.native("Connection", "close", |_, _, _| Ok(Value::ok(Value::Unit)));
    r.native("Row", "get", |_, v, a| {
        let k = arg_str(&a, 0, "get")?;
        Ok(Value::option(get_string(row(v)?, &k).ok().map(Value::str)))
    });
    r.native("Row", "get_string", |_, v, a| Ok(res(get_string(row(v)?, &arg_str(&a, 0, "get_string")?).map(Value::str))));
    r.native("Row", "get_int", |_, v, a| {
        let k = arg_str(&a, 0, "get_int")?;
        Ok(res(get_string(row(v)?, &k).and_then(|s| {
            s.parse::<i64>().map(|n| Value::Int(n as i128, IntTy::I64)).map_err(|e| format!("column {k} is not int: {e}"))
        })))
    });
    r.native("Row", "get_string_at", |_, v, a| Ok(res(get_at(row(v)?, arg_int(&a, 0, "get_string_at")?).map(Value::str))));
    r.native("Row", "get_int_at", |_, v, a| {
        let i = arg_int(&a, 0, "get_int_at")?;
        Ok(res(get_at(row(v)?, i).and_then(|s| {
            s.parse::<i64>().map(|n| Value::Int(n as i128, IntTy::I64)).map_err(|e| format!("column at {i} is not int: {e}"))
        })))
    });
}

/// The runtime keeps columns in a map: a repeated name holds the last column's value.
fn get_string(r: &Row, k: &str) -> Result<String, String> {
    r.0.iter().rev().find(|(n, _)| n == k).map(|(_, x)| x.clone()).ok_or_else(|| format!("column not found: {k}"))
}

fn get_at(r: &Row, i: i128) -> Result<String, String> {
    if i < 0 {
        return Err("index out of range".to_string());
    }
    let name = r.0.get(i as usize).map(|(n, _)| n.clone()).ok_or_else(|| format!("column index not found: {i}"))?;
    get_string(r, &name)
}

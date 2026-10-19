//! Database operations — Windjammer `std::db` contract (sqlx-backed, hidden from .wj users).

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use once_cell::sync::OnceCell;
use sqlx::pool::PoolConnection;
use sqlx::postgres::PgPoolOptions;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Column, Pool, Postgres, Row as SqlxRow, Sqlite, TypeInfo, ValueRef};

static RUNTIME: OnceCell<tokio::runtime::Runtime> = OnceCell::new();

fn runtime() -> Result<&'static tokio::runtime::Runtime, String> {
    if let Some(rt) = RUNTIME.get() {
        return Ok(rt);
    }
    let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    let _ = RUNTIME.set(rt);
    RUNTIME
        .get()
        .ok_or_else(|| "failed to init tokio runtime".to_string())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DatabaseType {
    SQLite,
    Postgres,
}

/// Process-wide Postgres pools, one per URL. Every `connect()` to the same URL shares the pool,
/// so connecting per request costs a pool checkout, not a TCP + auth handshake.
static PG_POOLS: OnceCell<Mutex<HashMap<String, Arc<Pool<Postgres>>>>> = OnceCell::new();

fn shared_pg_pool(url: &str) -> Result<Arc<Pool<Postgres>>, String> {
    let registry = PG_POOLS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut pools = registry.lock().map_err(|_| "pool registry poisoned".to_string())?;
    if let Some(p) = pools.get(url) {
        return Ok(p.clone());
    }
    let rt = runtime()?;
    let _guard = rt.enter();
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect_lazy(url)
        .map_err(|e| e.to_string())?;
    let pool = Arc::new(pool);
    pools.insert(url.to_string(), pool.clone());
    Ok(pool)
}

#[derive(Debug)]
enum DbBackend {
    #[allow(dead_code)]
    Postgres(Arc<Pool<Postgres>>),
    PostgresPending {
        url: String,
        pool: OnceCell<Arc<Pool<Postgres>>>,
    },
    Sqlite(Arc<Pool<Sqlite>>),
}

/// Database connection handle.
///
/// A Postgres `Connection` owns one pooled session, checked out on first use and kept until the
/// handle is dropped. Session state (`set_session`, `SET`, temp tables) and transactions
/// (`begin`/`commit`/`rollback`/`transaction`/`set_local`) therefore apply to every statement
/// issued through the same handle. Do not share one handle between threads that interleave
/// transactions.
#[derive(Debug)]
pub struct Connection {
    backend: DbBackend,
    session: Mutex<Option<PoolConnection<Postgres>>>,
    in_transaction: AtomicBool,
}

impl Connection {
    fn new(backend: DbBackend) -> Connection {
        Connection { backend, session: Mutex::new(None), in_transaction: AtomicBool::new(false) }
    }
}

/// Query result row — column access by name or index.
#[derive(Debug, Clone)]
pub struct Row {
    columns: HashMap<String, String>,
    ordered: Vec<String>,
}

impl Row {
    pub fn get(&self, column: &str) -> Option<String> {
        self.columns.get(column).cloned()
    }

    pub fn get_string(&self, column: impl AsRef<str>) -> Result<String, String> {
        let column = column.as_ref();
        self.columns
            .get(column)
            .cloned()
            .ok_or_else(|| format!("column not found: {column}"))
    }

    pub fn get_int(&self, column: impl AsRef<str>) -> Result<i64, String> {
        let column = column.as_ref();
        self.get_string(column)?
            .parse::<i64>()
            .map_err(|e| format!("column {column} is not int: {e}"))
    }

    pub fn get_string_at(&self, index: i64) -> Result<String, String> {
        let idx = usize::try_from(index).map_err(|_| "index out of range".to_string())?;
        self.ordered
            .get(idx)
            .and_then(|name| self.columns.get(name))
            .cloned()
            .ok_or_else(|| format!("column index not found: {index}"))
    }

    pub fn get_int_at(&self, index: i64) -> Result<i64, String> {
        self.get_string_at(index)?
            .parse::<i64>()
            .map_err(|e| format!("column at {index} is not int: {e}"))
    }
}

fn row_from_pg(row: &sqlx::postgres::PgRow) -> Row {
    let mut columns = HashMap::new();
    let mut ordered = Vec::new();
    for col in row.columns() {
        let name = col.name().to_string();
        let value = pg_cell_to_string(row, col.ordinal());
        ordered.push(name.clone());
        columns.insert(name, value);
    }
    Row { columns, ordered }
}

fn row_from_sqlite(row: &sqlx::sqlite::SqliteRow) -> Row {
    let mut columns = HashMap::new();
    let mut ordered = Vec::new();
    for col in row.columns() {
        let name = col.name().to_string();
        let value = sqlite_cell_to_string(row, col.ordinal());
        ordered.push(name.clone());
        columns.insert(name, value);
    }
    Row { columns, ordered }
}

fn pg_cell_to_string(row: &sqlx::postgres::PgRow, idx: usize) -> String {
    use sqlx::Row;
    let Ok(raw) = row.try_get_raw(idx) else {
        return String::new();
    };
    if raw.is_null() {
        return String::new();
    }
    let info = raw.type_info();
    let name = info.name();
    if name == "INT2" || name == "INT4" || name == "INT8" {
        return row
            .try_get::<i64, _>(idx)
            .map(|v| v.to_string())
            .unwrap_or_default();
    }
    if name == "BOOL" {
        return row
            .try_get::<bool, _>(idx)
            .map(|v| v.to_string())
            .unwrap_or_default();
    }
    row.try_get::<String, _>(idx)
        .or_else(|_| row.try_get::<&str, _>(idx).map(|s| s.to_string()))
        .unwrap_or_default()
}

fn sqlite_cell_to_string(row: &sqlx::sqlite::SqliteRow, idx: usize) -> String {
    use sqlx::Row;
    row.try_get::<String, _>(idx)
        .or_else(|_| row.try_get::<i64, _>(idx).map(|v| v.to_string()))
        .or_else(|_| row.try_get::<&str, _>(idx).map(|s| s.to_string()))
        .unwrap_or_default()
}

/// Connect to Postgres or SQLite from a URL / path string.
pub fn connect(url: impl AsRef<str>) -> Result<Connection, String> {
    crate::caps::db_open(url.as_ref());
    let url = url.as_ref();
    if url.starts_with("postgres://") || url.starts_with("postgresql://") {
        open_postgres(url)
    } else {
        open_sqlite(url)
    }
}

pub fn open(connection_string: &str) -> Result<Connection, String> {
    crate::caps::db_open(connection_string);
    connect(connection_string)
}

pub fn open_postgres(connection_string: &str) -> Result<Connection, String> {
    crate::caps::net_connect(connection_string);
    if !connection_string.starts_with("postgres://")
        && !connection_string.starts_with("postgresql://")
    {
        return Err(
            "PostgreSQL connection string must start with postgres:// or postgresql://".to_string(),
        );
    }
    // Lazy: pool is created on first query/execute (open succeeds without a live server).
    Ok(Connection::new(DbBackend::PostgresPending {
        url: connection_string.to_string(),
        pool: OnceCell::new(),
    }))
}

pub fn open_sqlite(path: &str) -> Result<Connection, String> {
    crate::caps::db_open(path);
    let url = if path == ":memory:" {
        "sqlite::memory:".to_string()
    } else if path.starts_with("sqlite:") {
        path.to_string()
    } else {
        format!("sqlite:{path}")
    };
    let rt = runtime()?;
    let pool = rt
        .block_on(async {
            SqlitePoolOptions::new()
                .max_connections(1)
                .connect(&url)
                .await
        })
        .map_err(|e| e.to_string())?;
    Ok(Connection::new(DbBackend::Sqlite(Arc::new(pool))))
}

impl Connection {
    fn postgres_pool(&self) -> Result<Arc<Pool<Postgres>>, String> {
        match &self.backend {
            DbBackend::Postgres(pool) => Ok(pool.clone()),
            DbBackend::PostgresPending { url, pool } => {
                if let Some(p) = pool.get() {
                    return Ok(p.clone());
                }
                let arc = shared_pg_pool(url)?;
                let _ = pool.set(arc.clone());
                Ok(arc)
            }
            _ => Err("not a postgres connection".to_string()),
        }
    }

    pub fn db_type(&self) -> &DatabaseType {
        static SQLITE: DatabaseType = DatabaseType::SQLite;
        static POSTGRES: DatabaseType = DatabaseType::Postgres;
        match &self.backend {
            DbBackend::Postgres(_) | DbBackend::PostgresPending { .. } => &POSTGRES,
            DbBackend::Sqlite(_) => &SQLITE,
        }
    }

    pub fn query(&self, sql: impl AsRef<str>, params: Vec<String>) -> Result<Vec<Row>, String> {
        let sql = sql.as_ref();
        let rt = runtime()?;
        match &self.backend {
            DbBackend::Postgres(_) | DbBackend::PostgresPending { .. } => {
                let pool = self.postgres_pool()?;
                let mut query = sqlx::query(sql);
                for p in params {
                    query = query.bind(p);
                }
                let rows = rt
                    .block_on(async {
                        let mut guard = self.session.lock().map_err(|_| sqlx::Error::PoolClosed)?;
                        if guard.is_none() {
                            *guard = Some(pool.acquire().await?);
                        }
                        let conn = guard.as_mut().ok_or(sqlx::Error::PoolClosed)?;
                        query.fetch_all(&mut **conn).await
                    })
                    .map_err(|e| e.to_string())?;
                Ok(rows.iter().map(row_from_pg).collect())
            }
            DbBackend::Sqlite(pool) => {
                let mut query = sqlx::query(sql);
                for p in params {
                    query = query.bind(p);
                }
                let rows = rt
                    .block_on(async { query.fetch_all(pool.as_ref()).await })
                    .map_err(|e| e.to_string())?;
                Ok(rows.iter().map(row_from_sqlite).collect())
            }
        }
    }

    pub fn execute(&self, sql: impl AsRef<str>, params: Vec<String>) -> Result<i64, String> {
        let sql = sql.as_ref();
        let rt = runtime()?;
        match &self.backend {
            DbBackend::Postgres(_) | DbBackend::PostgresPending { .. } => {
                let pool = self.postgres_pool()?;
                let mut query = sqlx::query(sql);
                for p in params {
                    query = query.bind(p);
                }
                let result = rt
                    .block_on(async {
                        let mut guard = self.session.lock().map_err(|_| sqlx::Error::PoolClosed)?;
                        if guard.is_none() {
                            *guard = Some(pool.acquire().await?);
                        }
                        let conn = guard.as_mut().ok_or(sqlx::Error::PoolClosed)?;
                        query.execute(&mut **conn).await
                    })
                    .map_err(|e| e.to_string())?;
                Ok(result.rows_affected() as i64)
            }
            DbBackend::Sqlite(pool) => {
                let mut query = sqlx::query(sql);
                for p in params {
                    query = query.bind(p);
                }
                let result = rt
                    .block_on(async { query.execute(pool.as_ref()).await })
                    .map_err(|e| e.to_string())?;
                Ok(result.rows_affected() as i64)
            }
        }
    }

    pub fn close(self) -> Result<(), String> {
        Ok(())
    }

    /// Start a transaction on this handle's session.
    pub fn begin(&self) -> Result<(), String> {
        if self.in_transaction.load(Ordering::SeqCst) {
            return Err("transaction already in progress".to_string());
        }
        self.execute("BEGIN", vec![])?;
        self.in_transaction.store(true, Ordering::SeqCst);
        Ok(())
    }

    pub fn commit(&self) -> Result<(), String> {
        if !self.in_transaction.load(Ordering::SeqCst) {
            return Err("no transaction in progress".to_string());
        }
        self.execute("COMMIT", vec![])?;
        self.in_transaction.store(false, Ordering::SeqCst);
        Ok(())
    }

    pub fn rollback(&self) -> Result<(), String> {
        if !self.in_transaction.load(Ordering::SeqCst) {
            return Err("no transaction in progress".to_string());
        }
        let r = self.execute("ROLLBACK", vec![]);
        self.in_transaction.store(false, Ordering::SeqCst);
        r.map(|_| ())
    }

    /// Run `body` inside a transaction: commit when it returns `Ok`, roll back when it returns `Err`.
    pub fn transaction<T>(
        &self,
        body: impl FnOnce(&Connection) -> Result<T, String>,
    ) -> Result<T, String> {
        self.begin()?;
        match body(self) {
            Ok(v) => {
                self.commit()?;
                Ok(v)
            }
            Err(e) => {
                let _ = self.rollback();
                Err(e)
            }
        }
    }

    /// Transaction-scoped setting (`SET LOCAL`), e.g. a row-level-security context. Postgres only;
    /// requires an open transaction.
    pub fn set_local(&self, key: impl AsRef<str>, value: impl AsRef<str>) -> Result<(), String> {
        if !self.in_transaction.load(Ordering::SeqCst) {
            return Err("set_local requires an open transaction".to_string());
        }
        self.set_config(key.as_ref(), value.as_ref(), true)
    }

    /// Session-scoped setting (`SET`), visible to every later statement on this handle. Postgres only.
    pub fn set_session(&self, key: impl AsRef<str>, value: impl AsRef<str>) -> Result<(), String> {
        self.set_config(key.as_ref(), value.as_ref(), false)
    }

    fn set_config(&self, key: &str, value: &str, local: bool) -> Result<(), String> {
        if !matches!(self.backend, DbBackend::Postgres(_) | DbBackend::PostgresPending { .. }) {
            return Err("session settings are only supported for postgres".to_string());
        }
        let sql = if local { "SELECT set_config($1, $2, true)" } else { "SELECT set_config($1, $2, false)" };
        self.query(sql, vec![key.to_string(), value.to_string()]).map(|_| ())
    }

    pub fn in_transaction(&self) -> bool {
        self.in_transaction.load(Ordering::SeqCst)
    }
}

impl Drop for Connection {
    /// Return the pinned session to the pool clean: an open transaction or leftover session state
    /// must never reach the next user of the pooled connection.
    fn drop(&mut self) {
        let in_async = tokio::runtime::Handle::try_current().is_ok();
        // Returning a pooled connection spawns work, so it needs a Tokio context even from sync code.
        // Declared before `conn` so the context outlives it.
        let _enter = if in_async { None } else { runtime().ok().map(|rt| rt.enter()) };
        let Ok(mut guard) = self.session.lock() else { return };
        let Some(mut conn) = guard.take() else { return };
        let clean = !in_async
            && !self.in_transaction.load(Ordering::SeqCst)
            && runtime()
                .ok()
                .map(|rt| rt.block_on(async { sqlx::query("DISCARD ALL").execute(&mut *conn).await }).is_ok())
                .unwrap_or(false);
        if !clean {
            conn.close_on_drop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqlite_memory_query_roundtrip() {
        let conn = open_sqlite(":memory:").expect("open");
        conn.execute("CREATE TABLE items (id INTEGER, name TEXT)", vec![])
            .expect("create");
        conn.execute(
            "INSERT INTO items (id, name) VALUES (?, ?)",
            vec!["1".into(), "Alice".into()],
        )
        .expect("insert");
        let rows = conn
            .query("SELECT id, name FROM items WHERE id = ?", vec!["1".into()])
            .expect("query");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].get_int("id").unwrap(), 1);
        assert_eq!(rows[0].get_string("name").unwrap(), "Alice");
    }

    fn sqlite_with_table() -> Connection {
        let conn = open_sqlite(":memory:").expect("open");
        conn.execute("CREATE TABLE t (id INTEGER)", vec![]).expect("create");
        conn
    }

    fn count(conn: &Connection) -> i64 {
        conn.query("SELECT COUNT(*) AS n FROM t", vec![]).unwrap()[0].get_int("n").unwrap()
    }

    #[test]
    fn transaction_commit_keeps_all_statements() {
        let conn = sqlite_with_table();
        conn.transaction(|tx| {
            tx.execute("INSERT INTO t (id) VALUES (?)", vec!["1".into()])?;
            tx.execute("INSERT INTO t (id) VALUES (?)", vec!["2".into()])?;
            Ok(())
        })
        .unwrap();
        assert_eq!(count(&conn), 2);
        assert!(!conn.in_transaction());
    }

    #[test]
    fn transaction_error_rolls_back_every_statement() {
        let conn = sqlite_with_table();
        let r: Result<(), String> = conn.transaction(|tx| {
            tx.execute("INSERT INTO t (id) VALUES (?)", vec!["1".into()])?;
            tx.execute("INSERT INTO nope (id) VALUES (?)", vec!["2".into()])?;
            Ok(())
        });
        assert!(r.is_err());
        assert_eq!(count(&conn), 0, "the first insert must be rolled back");
        assert!(!conn.in_transaction());
    }

    #[test]
    fn explicit_begin_rollback_and_misuse() {
        let conn = sqlite_with_table();
        assert!(conn.commit().is_err(), "commit without begin");
        conn.begin().unwrap();
        assert!(conn.begin().is_err(), "nested begin");
        conn.execute("INSERT INTO t (id) VALUES (?)", vec!["1".into()]).unwrap();
        conn.rollback().unwrap();
        assert_eq!(count(&conn), 0);
    }

    #[test]
    fn set_local_needs_transaction_and_postgres() {
        let conn = sqlite_with_table();
        assert!(conn.set_local("app.x", "1").is_err());
        conn.begin().unwrap();
        assert!(conn.set_local("app.x", "1").is_err(), "sqlite has no session settings");
        conn.rollback().unwrap();
    }

    #[test]
    fn postgres_connects_share_one_pool_per_url() {
        let url = "postgres://u:p@127.0.0.1:1/shared_pool_test";
        let a = open_postgres(url).unwrap();
        let b = open_postgres(url).unwrap();
        let other = open_postgres("postgres://u:p@127.0.0.1:1/other_db").unwrap();
        let (pa, pb, po) = (
            a.postgres_pool().unwrap(),
            b.postgres_pool().unwrap(),
            other.postgres_pool().unwrap(),
        );
        assert!(Arc::ptr_eq(&pa, &pb), "same URL must share a pool");
        assert!(!Arc::ptr_eq(&pa, &po), "different URL must not share a pool");
    }

    /// Needs a live server: `WJ_TEST_POSTGRES_URL=postgres://user@host/db cargo test -p windjammer-runtime --features db`.
    #[test]
    fn postgres_session_state_and_transactions() {
        let Ok(url) = std::env::var("WJ_TEST_POSTGRES_URL") else { return };
        let conn = connect(&url).unwrap();
        conn.set_session("app.tenant", "acme").unwrap();
        let pid1 = conn.query("SELECT pg_backend_pid()::text AS p", vec![]).unwrap()[0].get_string("p").unwrap();
        let pid2 = conn.query("SELECT pg_backend_pid()::text AS p", vec![]).unwrap()[0].get_string("p").unwrap();
        assert_eq!(pid1, pid2, "one handle is one session");
        let t = conn.query("SELECT current_setting('app.tenant') AS t", vec![]).unwrap();
        assert_eq!(t[0].get_string("t").unwrap(), "acme", "session setting must persist across statements");

        conn.execute("CREATE TEMP TABLE wj_tx (id INT)", vec![]).unwrap();
        let r: Result<(), String> = conn.transaction(|tx| {
            tx.set_local("app.tenant", "local")?;
            let t = tx.query("SELECT current_setting('app.tenant') AS t", vec![])?;
            assert_eq!(t[0].get_string("t").unwrap(), "local");
            tx.execute("INSERT INTO wj_tx VALUES (1)", vec![])?;
            Err("abort".to_string())
        });
        assert!(r.is_err());
        let n = conn.query("SELECT COUNT(*)::text AS n FROM wj_tx", vec![]).unwrap();
        assert_eq!(n[0].get_string("n").unwrap(), "0", "rolled back");
        let t = conn.query("SELECT current_setting('app.tenant') AS t", vec![]).unwrap();
        assert_eq!(t[0].get_string("t").unwrap(), "acme", "SET LOCAL ends with the transaction");
        drop(conn);

        // a new handle must not inherit the previous handle's session state
        let fresh = connect(&url).unwrap();
        let t = fresh.query("SELECT coalesce(current_setting('app.tenant', true), '') AS t", vec![]).unwrap();
        assert_eq!(t[0].get_string("t").unwrap(), "", "pooled session was reset on drop");
    }

    #[test]
    fn open_postgres_rejects_invalid_url() {
        assert!(open_postgres("invalid://x").is_err());
    }
}

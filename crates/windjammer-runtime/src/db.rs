//! Database operations — Windjammer `std::db` contract (sqlx-backed, hidden from .wj users).

use std::collections::HashMap;
use std::sync::Arc;

use once_cell::sync::OnceCell;
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

#[derive(Debug)]
enum DbBackend {
    Postgres(Arc<Pool<Postgres>>),
    PostgresPending {
        url: String,
        pool: OnceCell<Arc<Pool<Postgres>>>,
    },
    Sqlite(Arc<Pool<Sqlite>>),
}

/// Database connection (pool handle).
#[derive(Debug)]
pub struct Connection {
    backend: DbBackend,
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
    let url = url.as_ref();
    if url.starts_with("postgres://") || url.starts_with("postgresql://") {
        open_postgres(url)
    } else {
        open_sqlite(url)
    }
}

pub fn open(connection_string: &str) -> Result<Connection, String> {
    connect(connection_string)
}

pub fn open_postgres(connection_string: &str) -> Result<Connection, String> {
    if !connection_string.starts_with("postgres://")
        && !connection_string.starts_with("postgresql://")
    {
        return Err(
            "PostgreSQL connection string must start with postgres:// or postgresql://".to_string(),
        );
    }
    // Lazy: pool is created on first query/execute (open succeeds without a live server).
    Ok(Connection {
        backend: DbBackend::PostgresPending {
            url: connection_string.to_string(),
            pool: OnceCell::new(),
        },
    })
}

pub fn open_sqlite(path: &str) -> Result<Connection, String> {
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
    Ok(Connection {
        backend: DbBackend::Sqlite(Arc::new(pool)),
    })
}

impl Connection {
    fn postgres_pool(&self) -> Result<Arc<Pool<Postgres>>, String> {
        match &self.backend {
            DbBackend::Postgres(pool) => Ok(pool.clone()),
            DbBackend::PostgresPending { url, pool } => {
                if let Some(p) = pool.get() {
                    return Ok(p.clone());
                }
                let rt = runtime()?;
                let url = url.clone();
                let created = rt
                    .block_on(async { PgPoolOptions::new().max_connections(5).connect(&url).await })
                    .map_err(|e| e.to_string())?;
                let arc = Arc::new(created);
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
                    .block_on(async { query.fetch_all(pool.as_ref()).await })
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

    pub fn execute(&self, sql: impl AsRef<str>, params: Vec<String>) -> Result<u64, String> {
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
                    .block_on(async { query.execute(pool.as_ref()).await })
                    .map_err(|e| e.to_string())?;
                Ok(result.rows_affected())
            }
            DbBackend::Sqlite(pool) => {
                let mut query = sqlx::query(sql);
                for p in params {
                    query = query.bind(p);
                }
                let result = rt
                    .block_on(async { query.execute(pool.as_ref()).await })
                    .map_err(|e| e.to_string())?;
                Ok(result.rows_affected())
            }
        }
    }

    pub fn close(self) -> Result<(), String> {
        Ok(())
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

    #[test]
    fn open_postgres_rejects_invalid_url() {
        assert!(open_postgres("invalid://x").is_err());
    }
}

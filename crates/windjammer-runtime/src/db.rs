//! Database operations (SQLite and PostgreSQL)
//!
//! Windjammer's `std::db` module maps to these functions.
//! Note: This is a simplified wrapper. For production use, consider using sqlx directly.

use std::collections::HashMap;

/// Database type
#[derive(Debug, Clone, PartialEq)]
pub enum DatabaseType {
    SQLite,
    Postgres,
}

/// Database connection
#[derive(Debug)]
pub struct Connection {
    connection_string: String,
    db_type: DatabaseType,
}

/// Database row
#[derive(Debug, Clone)]
pub struct Row {
    pub columns: HashMap<String, String>,
}

impl Row {
    /// Get column value as string
    pub fn get(&self, column: &str) -> Option<String> {
        self.columns.get(column).cloned()
    }

    /// Get column value as integer
    pub fn get_int(&self, column: &str) -> Option<i64> {
        self.get(column)?.parse().ok()
    }

    /// Get column value as float
    pub fn get_float(&self, column: &str) -> Option<f64> {
        self.get(column)?.parse().ok()
    }

    /// Get column value as boolean
    pub fn get_bool(&self, column: &str) -> Option<bool> {
        self.get(column)?.parse().ok()
    }
}

/// Open a SQLite database connection
pub fn open_sqlite(path: &str) -> Result<Connection, String> {
    Ok(Connection {
        connection_string: path.to_string(),
        db_type: DatabaseType::SQLite,
    })
}

/// Open a PostgreSQL database connection
/// Format: "postgres://user:password@host:port/database"
pub fn open_postgres(connection_string: &str) -> Result<Connection, String> {
    if !connection_string.starts_with("postgres://") && !connection_string.starts_with("postgresql://") {
        return Err("PostgreSQL connection string must start with postgres:// or postgresql://".to_string());
    }
    
    Ok(Connection {
        connection_string: connection_string.to_string(),
        db_type: DatabaseType::Postgres,
    })
}

/// Open a database connection (auto-detect type)
pub fn open(connection_string: &str) -> Result<Connection, String> {
    if connection_string.starts_with("postgres://") || connection_string.starts_with("postgresql://") {
        open_postgres(connection_string)
    } else {
        open_sqlite(connection_string)
    }
}

impl Connection {
    /// Get database type
    pub fn db_type(&self) -> &DatabaseType {
        &self.db_type
    }

    /// Get connection string
    pub fn connection_string(&self) -> &str {
        &self.connection_string
    }

    /// Execute a query that returns rows
    pub fn query(&self, sql: &str, _params: &[String]) -> Result<Vec<Row>, String> {
        // Placeholder implementation
        // In a full implementation, this would use sqlx to execute the query
        Err(format!(
            "Database query not yet fully implemented. SQL: {} (type: {:?})",
            sql, self.db_type
        ))
    }

    /// Execute a statement that doesn't return rows (INSERT, UPDATE, DELETE)
    pub fn execute(&self, sql: &str, _params: &[String]) -> Result<u64, String> {
        // Placeholder implementation
        // In a full implementation, this would use sqlx to execute the statement
        Err(format!(
            "Database execute not yet fully implemented. SQL: {} (type: {:?})",
            sql, self.db_type
        ))
    }

    /// Begin a transaction
    pub fn begin_transaction(&self) -> Result<(), String> {
        Err("Transactions not yet implemented".to_string())
    }

    /// Commit a transaction
    pub fn commit(&self) -> Result<(), String> {
        Err("Transactions not yet implemented".to_string())
    }

    /// Rollback a transaction
    pub fn rollback(&self) -> Result<(), String> {
        Err("Transactions not yet implemented".to_string())
    }

    /// Close the connection
    pub fn close(self) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_sqlite() {
        let conn = open_sqlite(":memory:");
        assert!(conn.is_ok());
        let conn = conn.unwrap();
        assert_eq!(conn.db_type(), &DatabaseType::SQLite);
    }

    #[test]
    fn test_open_postgres() {
        let conn = open_postgres("postgres://user:pass@localhost/db");
        assert!(conn.is_ok());
        let conn = conn.unwrap();
        assert_eq!(conn.db_type(), &DatabaseType::Postgres);
    }

    #[test]
    fn test_open_postgres_invalid() {
        let conn = open_postgres("invalid://connection");
        assert!(conn.is_err());
    }

    #[test]
    fn test_open_auto_detect() {
        // Should detect SQLite
        let sqlite_conn = open(":memory:").unwrap();
        assert_eq!(sqlite_conn.db_type(), &DatabaseType::SQLite);

        // Should detect Postgres
        let pg_conn = open("postgres://localhost/db").unwrap();
        assert_eq!(pg_conn.db_type(), &DatabaseType::Postgres);

        // Should detect Postgres with postgresql:// prefix
        let pg_conn2 = open("postgresql://localhost/db").unwrap();
        assert_eq!(pg_conn2.db_type(), &DatabaseType::Postgres);
    }

    #[test]
    fn test_row_get() {
        let mut columns = HashMap::new();
        columns.insert("name".to_string(), "Alice".to_string());
        columns.insert("age".to_string(), "30".to_string());
        columns.insert("active".to_string(), "true".to_string());
        columns.insert("score".to_string(), "95.5".to_string());
        
        let row = Row { columns };
        assert_eq!(row.get("name"), Some("Alice".to_string()));
        assert_eq!(row.get_int("age"), Some(30));
        assert_eq!(row.get_bool("active"), Some(true));
        assert_eq!(row.get_float("score"), Some(95.5));
    }

    #[test]
    fn test_query_placeholder() {
        let conn = open(":memory:").unwrap();
        let result = conn.query("SELECT * FROM users", &[]);
        // Should return error for now (not fully implemented)
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_placeholder() {
        let conn = open_postgres("postgres://localhost/test").unwrap();
        let result = conn.execute("INSERT INTO users (name) VALUES ('Alice')", &[]);
        // Should return error for now (not fully implemented)
        assert!(result.is_err());
    }
}


struct Connection {
}

struct QueryBuilder {
}

struct Row {
}

struct Transaction {
}

impl Connection {
async fn execute(self, sql: &String) -> QueryBuilder {
        QueryBuilder {  }
}
async fn query(self, sql: &String) -> QueryBuilder {
        QueryBuilder {  }
}
async fn close(self) {
}
}

impl QueryBuilder {
#[inline]
fn bind<T>(self, value: &T) -> QueryBuilder {
        self
}
async fn execute(self, sql: &String) -> QueryBuilder {
        QueryBuilder {  }
}
async fn fetch_all(self) -> Result<Vec<Row>, String> {
        Ok(vec![])
}
async fn fetch_one(self) -> Result<Row, String> {
        Err("Not yet implemented")
}
}

impl Row {
#[inline]
fn get<T>(self, index: i64) -> Result<T, String> {
        Err("Not yet implemented")
}
#[inline]
fn get_by_name<T>(self, name: &String) -> Result<T, String> {
        Err("Not yet implemented")
}
}

async fn connect(url: &String) -> Result<Connection, String> {
    Err("Database connections will work after parser improvements in v0.14.0")
}


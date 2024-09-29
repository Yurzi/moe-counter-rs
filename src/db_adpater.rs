use std::{collections::HashMap, error::Error};

pub trait KVDBClient {
    type Value;
    fn init(&mut self) -> Result<(), Box<dyn Error>>;
    fn get(&self, key: &str) -> Option<Self::Value>;
    fn set(&mut self, key: &str, value: Self::Value) -> Result<(), Box<dyn Error>>;
    fn list(&self) -> Result<HashMap<String, Self::Value>, Box<dyn Error>>;
}

struct SqliteClient {
    db_path: String,
    table_name: String,
    connection: rusqlite::Connection,
}

impl SqliteClient {
    pub fn new(path: &str, table_name: &str) -> Self {
        let connection =
            rusqlite::Connection::open(path).expect(&format!("failed to open db on {}", path));

        SqliteClient {
            db_path: path.to_string(),
            table_name: table_name.to_string(),
            connection,
        }
    }
}

impl KVDBClient for SqliteClient {
    type Value = u64;
    fn init(&mut self) -> Result<(), Box<dyn Error>> {
        let sql = format!(
            "CREATE TABLE IF NOT EXISTS {} (
                key TEXT NOT NULL UNIQUE,
                value INTEGER NOT NULL
            )",
            self.table_name
        );

        let _ret = self.connection.execute(&sql, ())?;
        Ok(())
    }

    fn get(&self, key: &str) -> Option<Self::Value> {
        let sql = format!("SELECT value FROM {} WHERE key = ?1", self.table_name);
        let stmt = self.connection.prepare(&sql);
        if stmt.is_err() {
            return None;
        }
        let mut stmt = stmt.unwrap();
        let value_iter = stmt.query_map(rusqlite::params![key], |row| row.get::<_, Self::Value>(0));

        if value_iter.is_err() {
            return None;
        }

        let value_iter = value_iter.unwrap();

        // actually key is unqiue, so just iter all and sum.
        let mut res = 0;
        for value in value_iter {
            res += match value {
                Ok(val) => val,
                Err(_) => 0,
            }
        }

        Some(res)
    }

    fn set(&mut self, key: &str, value: Self::Value) -> Result<(), Box<dyn Error>> {
        let sql = format!("INSERT INTO {} (key, value) VALUES (?1. ?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value", self.table_name);
        let _ret = self
            .connection
            .execute(&sql, rusqlite::params![key, value])?;

        Ok(())
    }

    fn list(&self) -> Result<HashMap<String, Self::Value>, Box<dyn Error>> {
        let sql = format!("SELECT key, value FROM {}", self.table_name);
        let mut stmt = self.connection.prepare(&sql)?;
        let mut kv_map = HashMap::new();
        let mut rows = stmt.query([])?;

        while let Some(row) = rows.next()? {
            let key: String = row.get(0)?;
            let value: Self::Value = row.get(1)?;
            kv_map.insert(key, value);
        }

        Ok(kv_map)
    }
}

struct DBManager {
    backend: Box<dyn KVDBClient<Value = u64>>,
    cache_db: Option<Box<dyn KVDBClient<Value = u64>>>,
}

impl DBManager {
    pub fn new(
        backend: Box<dyn KVDBClient<Value = u64>>,
        cache_db: Option<Box<dyn KVDBClient<Value = u64>>>,
    ) -> Self {
        DBManager { backend, cache_db }
    }

    pub fn init(&mut self) -> Result<(), Box<dyn Error>> {
        self.backend.init()?;
        if self.cache_db.is_some() {
            self.cache_db.as_mut().unwrap().init()?;
        }

        Ok(())
    }

    fn conut_on_backend(&mut self, key: &str) -> Option<u64> {
        // get count on db
        let prev_count = self.backend.get(key).unwrap_or(0);

        // add count
        let now_count = prev_count.saturating_add(1);

        // set count to db
        let ret = self.backend.set(key, now_count);

        if ret.is_err() {
            return None;
        }

        Some(now_count)
    }

    pub fn count(&mut self, key: &str) -> Option<u64> {
        if self.cache_db.is_none() {
            return self.conut_on_backend(key);
        }

        unimplemented!()
    }
}

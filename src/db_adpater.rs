use std::sync::Arc;
use std::{collections::HashMap, error::Error};
use tokio::sync::Mutex;

pub trait KVDBClient: Send + Sync {
    type Value;
    async fn init(&self) -> Result<(), Box<dyn Error>>;
    async fn get(&self, key: &str) -> Option<Self::Value>;
    async fn set(&self, key: &str, value: Self::Value) -> Result<(), Box<dyn Error>>;
}

pub struct SqliteClient {
    table_name: String,
    connection: Arc<Mutex<rusqlite::Connection>>,
}

impl SqliteClient {
    pub fn new(path: &str, table_name: &str) -> Self {
        let connection =
            rusqlite::Connection::open(path).expect(&format!("failed to open db on {}", path));

        SqliteClient {
            table_name: table_name.to_string(),
            connection: Arc::new(Mutex::new(connection)),
        }
    }
}

impl KVDBClient for SqliteClient {
    type Value = u64;
    async fn init(&self) -> Result<(), Box<dyn Error>> {
        let sql = format!(
            "CREATE TABLE IF NOT EXISTS {} (
                key TEXT NOT NULL UNIQUE,
                value INTEGER NOT NULL
            )",
            self.table_name
        );

        let _ret = self.connection.lock().await.execute(&sql, ())?;
        Ok(())
    }

    async fn get(&self, key: &str) -> Option<Self::Value> {
        let sql = format!("SELECT value FROM {} WHERE key = ?1", self.table_name);
        let conn = self.connection.lock().await;
        let stmt = conn.prepare(&sql);
        if stmt.is_err() {
            return None;
        }
        let mut stmt = stmt.unwrap();
        let value_iter = stmt.query_map(rusqlite::params![key], |row| row.get::<_, Self::Value>(0));

        if value_iter.is_err() {
            return None;
        }

        let mut value_iter = value_iter.unwrap();
        // actually key is unqiue, so just iter all and sum.
        let ret = match value_iter.next() {
            Some(val) => match val {
                Ok(value) => Some(value),
                Err(_) => None,
            },
            None => None,
        };

        ret
    }

    async fn set(&self, key: &str, value: Self::Value) -> Result<(), Box<dyn Error>> {
        let sql = format!("INSERT INTO {} (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value", self.table_name);
        let _ret = self
            .connection
            .lock()
            .await
            .execute(&sql, rusqlite::params![key, value])?;

        Ok(())
    }
}

pub struct DBManager {
    cache: Arc<Mutex<HashMap<String, u64>>>,
    backend: SqliteClient,
}

impl DBManager {
    pub fn new(backend: SqliteClient) -> Self {
        DBManager {
            cache: Arc::new(Mutex::new(HashMap::new())),
            backend,
        }
    }

    pub async fn init(&mut self) -> Result<(), Box<dyn Error>> {
        self.backend.init().await
    }

    async fn count_on_cache(&self, key: &str) -> Option<u64> {
        // key must exist
        let mut cache = self.cache.lock().await;
        let prev_count = cache.get(key).unwrap();

        let now_count = prev_count.saturating_add(1);

        // set count to cache
        cache.insert(key.to_string(), now_count);

        Some(now_count)
    }

    async fn load_to_cache(&self, key: &str, value: u64) {
        self.cache.lock().await.insert(key.to_string(), value);
    }

    async fn check_in_cache(&self, key: &str) -> bool {
        self.cache.lock().await.get(key).is_some()
    }

    pub async fn count(&self, key: &str) -> Option<u64> {
        // check in cache
        // in in cache
        let exist_in_cache = self.check_in_cache(key).await;
        if exist_in_cache {
            // count on cache
            return self.count_on_cache(key).await;
        }

        // if not in cache
        // found key on db, if not key on db, then think the value is 0
        let value = self.backend.get(key).await.unwrap_or(0);
        self.load_to_cache(key, value).await;

        // count in cache
        self.count_on_cache(key).await
    }

    pub async fn sync_to_backend(&self) -> Result<(), Box<dyn Error>> {
        for (key, value) in self.cache.lock().await.iter() {
            let _ = self.backend.set(key, *value).await;
        }

        Ok(())
    }
}

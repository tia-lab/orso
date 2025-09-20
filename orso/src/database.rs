use crate::Error;
use anyhow::Result;
use libsql::{Builder, Database as LibsqlDatabase, Rows};
use serde::{Deserialize, Serialize};
use tracing::debug;

#[cfg(feature = "sqlite")]
use rusqlite::Connection as RusqliteConnection;
#[cfg(feature = "sqlite")]
use std::sync::Arc;
#[cfg(feature = "sqlite")]
use std::sync::Mutex;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub mode: TursoMode,
    pub local_db_path: String,
    pub db_url: String,
    pub db_token: String,
}

impl DatabaseConfig {
    pub fn new(mode: TursoMode, local_db_path: String, db_url: String, db_token: String) -> Self {
        Self {
            mode,
            local_db_path,
            db_url,
            db_token,
        }
    }

    pub fn memory() -> Self {
        Self {
            mode: TursoMode::Memory,
            local_db_path: String::new(),
            db_url: String::new(),
            db_token: String::new(),
        }
    }

    pub fn local(db_path: impl Into<String>) -> Self {
        Self {
            mode: TursoMode::Local,
            local_db_path: db_path.into(),
            db_url: String::new(),
            db_token: String::new(),
        }
    }

    #[cfg(feature = "sqlite")]
    pub fn sqlite(db_path: impl Into<String>) -> Self {
        Self {
            mode: TursoMode::Local,
            local_db_path: db_path.into(),
            db_url: String::new(),
            db_token: String::new(),
        }
    }

    pub fn remote(db_url: impl Into<String>, db_token: impl Into<String>) -> Self {
        Self {
            mode: TursoMode::Remote,
            local_db_path: String::new(),
            db_url: db_url.into(),
            db_token: db_token.into(),
        }
    }

    pub fn sync(
        local_db_path: impl Into<String>,
        db_url: impl Into<String>,
        db_token: impl Into<String>,
    ) -> Self {
        Self {
            mode: TursoMode::Sync,
            local_db_path: local_db_path.into(),
            db_url: db_url.into(),
            db_token: db_token.into(),
        }
    }

    pub fn embed(
        local_db_path: impl Into<String>,
        db_url: impl Into<String>,
        db_token: impl Into<String>,
    ) -> Self {
        Self {
            mode: TursoMode::Embed,
            local_db_path: local_db_path.into(),
            db_url: db_url.into(),
            db_token: db_token.into(),
        }
    }
}

// Modes
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TursoMode {
    Memory,
    Local,
    Sync,
    Remote,
    Embed,
}

#[derive(Debug)]
pub struct Database {
    pub db: libsql::Database,
    pub conn: libsql::Connection,
    pub mode: TursoMode,
    #[cfg(feature = "sqlite")]
    pub sqlite_conn: Option<Arc<Mutex<RusqliteConnection>>>,
}

impl TursoMode {
    /// Returns the TursoMode based on the TURSO_MODE environment variable.
    pub fn from_env() -> Self {
        dotenv::dotenv().ok();
        match std::env::var("TURSO_MODE")
            .unwrap_or_else(|_| "local".to_string())
            .to_lowercase()
            .as_str()
        {
            "local" => TursoMode::Local,
            "sync" => TursoMode::Sync,
            "remote" => TursoMode::Remote,
            "embed" => TursoMode::Embed,
            _ => TursoMode::Local, // default fallback
        }
    }
}

impl Database {
    pub async fn init(config: DatabaseConfig) -> Result<Self> {
        let db = Self::client(config.clone()).await?;
        let conn = db.connect().map_err(|e| Error::Connection(e))?;
        let mode = config.mode.clone();

        // Enable foreign key constraints for SQLite
        conn.execute("PRAGMA foreign_keys = ON", ())
            .await
            .map_err(|e| Error::Connection(e))?;

        debug!("Turso database connection established with foreign keys enabled");
        
        #[cfg(feature = "sqlite")]
        let sqlite_conn = if matches!(config.mode, TursoMode::Local) && cfg!(feature = "sqlite") {
            Some(Arc::new(Mutex::new(RusqliteConnection::open(&config.local_db_path)
                .map_err(|e| Error::Connection(libsql::Error::ConnectionFailed(e.to_string())))?)))
        } else {
            None
        };

        Ok(Self { 
            db, 
            conn, 
            mode,
            #[cfg(feature = "sqlite")]
            sqlite_conn,
        })
    }
    
    // Initialize Turso client with ConfigManager integration - uses defaults if not in config.yaml
    async fn client(config: DatabaseConfig) -> Result<LibsqlDatabase, Error> {
        let local_db_path = config.local_db_path;
        let db_url = config.db_url;
        let db_token = config.db_token;
        let mode = config.mode;

        let db = match mode {
            TursoMode::Memory => Builder::new_local(":memory:")
                .build()
                .await
                .map_err(|e| Error::Connection(e))?,
            TursoMode::Local => Builder::new_local(&local_db_path)
                .build()
                .await
                .map_err(|e| Error::Connection(e))?,
            TursoMode::Sync => Builder::new_synced_database(local_db_path, db_url, db_token)
                .build()
                .await
                .map_err(|e| Error::Connection(e))?,
            TursoMode::Remote => Builder::new_remote(db_url, db_token)
                .build()
                .await
                .map_err(|e| Error::Connection(e))?,
            TursoMode::Embed => Builder::new_remote_replica(local_db_path, db_url, db_token)
                .build()
                .await
                .map_err(|e| Error::Connection(e))?,
        };
        Ok(db)
    }

    pub async fn sync(&self) -> Result<()> {
        if self.mode == TursoMode::Sync {
            self.db.sync().await.map_err(|e| Error::Connection(e))?;
            debug!("Turso database synced successfully");
        }
        Ok(())
    }

    pub async fn query(
        &self,
        sql: &str,
        params: Vec<libsql::Value>,
    ) -> Result<Rows, libsql::Error> {
        self.conn.query(sql, params).await
    }

    pub async fn execute(&self, sql: &str) -> Result<u64, libsql::Error> {
        self.conn.execute(sql, ()).await
    }
    
    // New method for SQLite operations
    #[cfg(feature = "sqlite")]
    pub fn sqlite_execute(&self, sql: &str) -> Result<usize, rusqlite::Error> {
        if let Some(conn) = &self.sqlite_conn {
            let conn = conn.lock().map_err(|_| rusqlite::Error::ExecuteReturnedResults)?;
            conn.execute(sql, [])
        } else {
            Err(rusqlite::Error::ExecuteReturnedResults)
        }
    }
    
    #[cfg(feature = "sqlite")]
    pub fn sqlite_query<T, F>(&self, sql: &str, f: F) -> Result<Vec<T>, rusqlite::Error>
    where
        F: FnMut(&rusqlite::Row) -> rusqlite::Result<T>,
    {
        if let Some(conn) = &self.sqlite_conn {
            let conn = conn.lock().map_err(|_| rusqlite::Error::QueryReturnedNoRows)?;
            let mut stmt = conn.prepare(sql)?;
            let rows = stmt.query_map([], f)?;
            rows.collect()
        } else {
            Err(rusqlite::Error::QueryReturnedNoRows)
        }
    }
}

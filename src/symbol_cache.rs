use std::path::Path;
use std::sync::Mutex;

use rusqlite::Connection;
use sha2::{Digest, Sha256};

use crate::treesitter::{ImportEdge, SupportedLanguage, Symbol};

const SCHEMA: &str = "\
    CREATE TABLE IF NOT EXISTS symbol_cache (\
        file_path TEXT PRIMARY KEY,\
        content_hash TEXT NOT NULL,\
        symbols_json TEXT NOT NULL,\
        imports_json TEXT NOT NULL,\
        updated_at TEXT NOT NULL\
    );\
";

pub struct SymbolCache {
    conn: Mutex<Connection>,
}

#[derive(Debug, Clone)]
pub struct CachedEntry {
    pub symbols: Vec<Symbol>,
    pub imports: Vec<ImportEdge>,
}

#[derive(Debug)]
pub enum SymbolCacheError {
    Db(rusqlite::Error),
    Parse(String),
    Serde(String),
}

impl std::fmt::Display for SymbolCacheError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Db(e) => write!(f, "symbol cache db: {e}"),
            Self::Parse(e) => write!(f, "symbol cache parse: {e}"),
            Self::Serde(e) => write!(f, "symbol cache serde: {e}"),
        }
    }
}

impl std::error::Error for SymbolCacheError {}

impl From<rusqlite::Error> for SymbolCacheError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Db(e)
    }
}

impl SymbolCache {
    /// # Errors
    /// Returns `SymbolCacheError` if the database cannot be opened.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, SymbolCacheError> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// # Errors
    /// Returns `SymbolCacheError` if the in-memory database cannot be created.
    pub fn open_in_memory() -> Result<Self, SymbolCacheError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Get cached entry or parse from source, caching the result.
    ///
    /// # Errors
    /// Returns `SymbolCacheError` on database or parse errors.
    pub fn get_or_parse(
        &self,
        file_path: &str,
        source: &str,
        lang: SupportedLanguage,
    ) -> Result<CachedEntry, SymbolCacheError> {
        let hash = content_hash(source);
        if let Some(entry) = self.get(file_path, &hash)? {
            return Ok(entry);
        }
        let symbols = crate::treesitter::extract_symbols(source, lang)
            .map_err(|e| SymbolCacheError::Parse(e.to_string()))?;
        let imports = crate::treesitter::extract_imports(source, lang, file_path)
            .map_err(|e| SymbolCacheError::Parse(e.to_string()))?;
        self.put(file_path, &hash, &symbols, &imports)?;
        Ok(CachedEntry { symbols, imports })
    }

    /// Invalidate a cache entry.
    ///
    /// # Errors
    /// Returns `SymbolCacheError` on database errors.
    pub fn invalidate(&self, file_path: &str) -> Result<(), SymbolCacheError> {
        self.conn
            .lock()
            .map_err(|e| SymbolCacheError::Parse(e.to_string()))?
            .execute("DELETE FROM symbol_cache WHERE file_path = ?1", [file_path])?;
        Ok(())
    }

    #[must_use]
    pub fn count(&self) -> usize {
        let Ok(conn) = self.conn.lock() else {
            return 0;
        };
        conn.query_row("SELECT COUNT(*) FROM symbol_cache", [], |row| {
            row.get::<_, usize>(0)
        })
        .unwrap_or(0)
    }

    fn get(
        &self,
        file_path: &str,
        expected_hash: &str,
    ) -> Result<Option<CachedEntry>, SymbolCacheError> {
        let result = {
            let conn = self
                .conn
                .lock()
                .map_err(|e| SymbolCacheError::Parse(e.to_string()))?;
            symbol_cache_get(&conn, file_path)?
        };
        match result {
            Some((hash, sym_json, imp_json)) => {
                if hash != expected_hash {
                    return Ok(None);
                }
                let symbols: Vec<Symbol> = serde_json::from_str(&sym_json)
                    .map_err(|e| SymbolCacheError::Serde(e.to_string()))?;
                let imports: Vec<ImportEdge> = serde_json::from_str(&imp_json)
                    .map_err(|e| SymbolCacheError::Serde(e.to_string()))?;
                Ok(Some(CachedEntry { symbols, imports }))
            }
            None => Ok(None),
        }
    }

    fn put(
        &self,
        file_path: &str,
        hash: &str,
        symbols: &[Symbol],
        imports: &[ImportEdge],
    ) -> Result<(), SymbolCacheError> {
        let sym_json =
            serde_json::to_string(symbols).map_err(|e| SymbolCacheError::Serde(e.to_string()))?;
        let imp_json =
            serde_json::to_string(imports).map_err(|e| SymbolCacheError::Serde(e.to_string()))?;
        let now = chrono::Utc::now().to_rfc3339();
        self.conn
            .lock()
            .map_err(|e| SymbolCacheError::Parse(e.to_string()))?
            .execute(
                "INSERT OR REPLACE INTO symbol_cache (file_path, content_hash, symbols_json, imports_json, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![file_path, hash, sym_json, imp_json, now],
            )?;
        Ok(())
    }
}

/// Query a single cache entry by file path.
fn symbol_cache_get(
    conn: &rusqlite::Connection,
    file_path: &str,
) -> Result<Option<(String, String, String)>, SymbolCacheError> {
    match conn.query_row(
        "SELECT content_hash, symbols_json, imports_json FROM symbol_cache WHERE file_path = ?1",
        [file_path],
        |row| {
            let hash: String = row.get(0)?;
            let sym_json: String = row.get(1)?;
            let imp_json: String = row.get(2)?;
            Ok((hash, sym_json, imp_json))
        },
    ) {
        Ok(tuple) => Ok(Some(tuple)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// SHA-256 based content hash (stable across Rust versions).
#[must_use]
pub fn content_hash(source: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(source.as_bytes());
    hex::encode(hasher.finalize())[..16].to_string()
}

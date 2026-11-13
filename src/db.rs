// src/db.rs
//! SQLite persistence layer for storing and managing watchlist ticker symbols.

use crate::ticker::{AssetCategory, Ticker};
use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};

/// SQLite-backed store for watchlist ticker symbols.
pub struct TickerDb {
    conn: Connection,
}

impl TickerDb {
    /// Returns the default database path (`~/.market_tui/tickers.db`).
    pub fn default_path() -> PathBuf {
        if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
            PathBuf::from(home).join(".market_tui").join("tickers.db")
        } else {
            PathBuf::from(".market_tui").join("tickers.db")
        }
    }

    /// Opens (or creates) the default SQLite database on disk and initializes its schema.
    pub fn open_default() -> Result<Self> {
        Self::open_path(Self::default_path())
    }

    /// Opens (or creates) a SQLite database at the given path and initializes its schema.
    pub fn open_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("failed to create db directory {:?}", parent))?;
            }
        }
        let conn = Connection::open(path)
            .with_context(|| format!("failed to open sqlite database {:?}", path))?;
        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    /// Opens an in-memory SQLite database (useful for tests).
    #[allow(dead_code)]
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS tickers (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                symbol TEXT NOT NULL UNIQUE,
                display_name TEXT NOT NULL,
                category TEXT NOT NULL,
                sort_order INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
            );
            CREATE TABLE IF NOT EXISTS app_meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            ",
        )?;
        Ok(())
    }

    /// Loads saved tickers, or seeds `defaults` on first run when the database is uninitialized.
    pub fn load_or_seed(&self, defaults: &[Ticker]) -> Result<Vec<Ticker>> {
        let existing = self.list_tickers()?;
        if !existing.is_empty() {
            return Ok(existing);
        }

        let seeded: Option<String> = self
            .conn
            .query_row(
                "SELECT value FROM app_meta WHERE key = 'seeded'",
                [],
                |row| row.get(0),
            )
            .ok();

        if seeded.is_some() {
            return Ok(Vec::new());
        }

        for ticker in defaults {
            self.add_ticker(ticker)?;
        }
        self.conn.execute(
            "INSERT OR REPLACE INTO app_meta (key, value) VALUES ('seeded', '1')",
            [],
        )?;

        self.list_tickers()
    }

    /// Lists all persisted tickers in display order.
    pub fn list_tickers(&self) -> Result<Vec<Ticker>> {
        let mut stmt = self.conn.prepare(
            "SELECT symbol, display_name, category FROM tickers ORDER BY sort_order ASC, id ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            let symbol: String = row.get(0)?;
            let display_name: String = row.get(1)?;
            let category_str: String = row.get(2)?;
            Ok(Ticker::from_stored(
                symbol,
                display_name,
                AssetCategory::from_label(&category_str),
            ))
        })?;

        let mut tickers = Vec::new();
        for item in rows {
            tickers.push(item?);
        }
        Ok(tickers)
    }

    /// Inserts a ticker symbol into the database if not already present.
    pub fn add_ticker(&self, ticker: &Ticker) -> Result<()> {
        let next_order: i64 = self.conn.query_row(
            "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM tickers",
            [],
            |row| row.get(0),
        )?;

        self.conn.execute(
            "INSERT OR IGNORE INTO tickers (symbol, display_name, category, sort_order)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                ticker.symbol,
                ticker.display_name,
                ticker.category.label(),
                next_order
            ],
        )?;
        Ok(())
    }

    /// Removes a ticker symbol from the database. Returns true if a row was deleted.
    pub fn remove_ticker(&self, symbol: &str) -> Result<bool> {
        let deleted = self
            .conn
            .execute("DELETE FROM tickers WHERE symbol = ?1", params![symbol])?;
        Ok(deleted > 0)
    }
}

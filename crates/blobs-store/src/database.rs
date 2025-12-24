//! SQLite database for blob store metadata.
//!
//! This module manages its own SQLite connection pool, separate from any
//! application database. The schema stores only metadata - all blob data
//! lives in object storage.

use std::path::Path;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::Row;
use thiserror::Error;
use tracing::info;

/// Database connection pool for blob store metadata.
#[derive(Debug, Clone)]
pub struct Database {
    pool: SqlitePool,
}

impl std::ops::Deref for Database {
    type Target = SqlitePool;

    fn deref(&self) -> &Self::Target {
        &self.pool
    }
}

/// Errors that can occur when setting up the database.
#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] sqlx::Error),

    #[error("Migration error: {0}")]
    Migration(String),
}

impl Database {
    /// Create a new database connection with a file-based SQLite database.
    ///
    /// The database file will be created if it doesn't exist.
    /// Migrations are run automatically.
    pub async fn new(path: impl AsRef<Path>) -> Result<Self, DatabaseError> {
        let path = path.as_ref();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                DatabaseError::Migration(format!("Failed to create database directory: {}", e))
            })?;
        }

        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;

        let db = Self { pool };
        db.run_migrations().await?;

        info!("Blob store database initialized at {:?}", path);
        Ok(db)
    }

    /// Create a new in-memory database.
    ///
    /// Useful for testing or ephemeral storage where metadata can be
    /// recovered from object storage on restart.
    pub async fn in_memory() -> Result<Self, DatabaseError> {
        let options = SqliteConnectOptions::new()
            .filename(":memory:")
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await?;

        let db = Self { pool };
        db.run_migrations().await?;

        info!("Blob store database initialized in-memory");
        Ok(db)
    }

    /// Run database migrations.
    async fn run_migrations(&self) -> Result<(), DatabaseError> {
        // Create blobs table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS blobs (
                hash TEXT PRIMARY KEY,
                size INTEGER NOT NULL,
                has_outboard INTEGER NOT NULL,
                state TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create tags table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS tags (
                name TEXT PRIMARY KEY,
                hash TEXT NOT NULL,
                format TEXT NOT NULL,
                created_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create index on tags.hash
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_tags_hash ON tags(hash)
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get the number of blobs in the database.
    pub async fn blob_count(&self) -> Result<i64, DatabaseError> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM blobs")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.get("count"))
    }

    /// Get the number of tags in the database.
    pub async fn tag_count(&self) -> Result<i64, DatabaseError> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM tags")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.get("count"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_memory_database() {
        let db = Database::in_memory().await.unwrap();
        assert_eq!(db.blob_count().await.unwrap(), 0);
        assert_eq!(db.tag_count().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_migrations_are_idempotent() {
        let db = Database::in_memory().await.unwrap();
        // Running migrations again should not fail
        db.run_migrations().await.unwrap();
    }
}

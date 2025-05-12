use std::sync::Arc;

use anyhow::Result;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{OptionalExtension, params};
use tokio::task;

use super::picture_dto::PictureDto;

pub struct PictureRepository {
    pool: Arc<Pool<SqliteConnectionManager>>,
}

impl PictureRepository {
    pub fn new(pool: Pool<SqliteConnectionManager>) -> Self {
        Self {
            pool: Arc::new(pool),
        }
    }

    pub fn init_schema(&self) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS pictures (
               id        TEXT PRIMARY KEY,
               filename  TEXT NOT NULL,
               added_at  INTEGER NOT NULL
            );
            "#,
        )?;
        Ok(())
    }

    pub async fn list(&self) -> Result<Vec<PictureDto>> {
        let pool = self.pool.clone();
        task::spawn_blocking(move || {
            let conn = pool.get()?;
            let mut stmt =
                conn.prepare("SELECT id, filename, added_at FROM pictures ORDER BY added_at DESC")?;

            let iter = stmt
                .query_map([], |row| {
                    Ok(PictureDto {
                        id: row.get(0)?,
                        filename: row.get(1)?,
                        added_at: row.get(2)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;

            Ok(iter)
        })
        .await?
    }

    pub async fn add(&self, filename: &str) -> Result<PictureDto> {
        let pool = self.pool.clone();
        let filename = filename.to_owned();
        task::spawn_blocking(move || {
            let conn = pool.get()?;

            let dto = PictureDto {
                id: uuid::Uuid::new_v4().to_string(),
                filename,
                added_at: chrono::Utc::now().timestamp_millis(),
            };

            conn.execute(
                "INSERT INTO pictures (id, filename, added_at) VALUES (?1, ?2, ?3)",
                params![dto.id, dto.filename, dto.added_at],
            )?;
            Ok(dto)
        })
        .await?
    }

    pub async fn delete(&self, id: &str) -> Result<bool> {
        let pool = self.pool.clone();
        let id = id.to_string();
        task::spawn_blocking(move || {
            let conn = pool.get()?;
            let affected = conn.execute("DELETE FROM pictures WHERE id = ?1", params![id])?;
            Ok(affected == 1)
        })
        .await?
    }

    pub async fn get(&self, id: &str) -> Result<Option<PictureDto>> {
        let pool = self.pool.clone();
        let id = id.to_string();
        task::spawn_blocking(move || {
            let conn = pool.get()?;
            conn.query_row(
                "SELECT id, filename, added_at FROM pictures WHERE id = ?1",
                params![id],
                |row| {
                    Ok(PictureDto {
                        id: row.get(0)?,
                        filename: row.get(1)?,
                        added_at: row.get(2)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
        })
        .await?
    }
}

use std::sync::Arc;

use anyhow::Result;
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use password_hash::rand_core::RngCore;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{OptionalExtension, params};
use tokio::task;

use super::dto::{KeyInfo, Picture};

pub struct Repository {
    pool: Arc<Pool<SqliteConnectionManager>>,
}

impl Repository {
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

            CREATE TABLE IF NOT EXISTS api_keys (
                id          TEXT PRIMARY KEY,
                token_hash  TEXT NOT NULL,
                scope       TEXT NOT NULL,
                created_at  INTEGER NOT NULL
            );
            "#,
        )?;
        Ok(())
    }

    pub async fn count_pictures(&self) -> anyhow::Result<usize> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            let n: usize = conn.query_row("SELECT COUNT(*) FROM pictures", [], |r| r.get(0))?;
            Ok(n)
        })
        .await?
    }

    pub async fn list_pictures(&self) -> Result<Vec<Picture>> {
        let pool = self.pool.clone();
        task::spawn_blocking(move || {
            let conn = pool.get()?;
            let mut stmt = conn.prepare(
                r#"
                SELECT id, filename, added_at
                FROM pictures
                ORDER BY added_at DESC
                "#,
            )?;

            let iter = stmt
                .query_map([], |row| {
                    Ok(Picture {
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

    pub async fn add_picture(&self, filename: &str) -> Result<Picture> {
        let pool = self.pool.clone();
        let filename = filename.to_owned();
        task::spawn_blocking(move || {
            let conn = pool.get()?;

            let dto = Picture {
                id: uuid::Uuid::new_v4().to_string(),
                filename,
                added_at: chrono::Utc::now().timestamp_millis(),
            };

            conn.execute(
                r#"
                INSERT INTO pictures (id, filename, added_at)
                VALUES (?1, ?2, ?3)
                "#,
                params![dto.id, dto.filename, dto.added_at],
            )?;
            Ok(dto)
        })
        .await?
    }

    /// Returns the filename *and* deletes the DB row in one round-trip.
    pub async fn delete_picture_and_return_filename(
        &self,
        id: &str,
    ) -> anyhow::Result<Option<String>> {
        let id = id.to_owned();
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            let fname: Option<String> = conn
                .query_row("SELECT filename FROM pictures WHERE id = ?1", [&id], |r| {
                    r.get(0)
                })
                .optional()?;
            if let Some(ref _f) = fname {
                conn.execute("DELETE FROM pictures WHERE id = ?1", [&id])?;
            }
            Ok(fname)
        })
        .await?
    }

    pub async fn get_picture(&self, id: &str) -> Result<Option<Picture>> {
        let pool = self.pool.clone();
        let id = id.to_string();
        task::spawn_blocking(move || {
            let conn = pool.get()?;
            conn.query_row(
                r#"
                SELECT id, filename, added_at
                FROM pictures
                WHERE id = ?1
                "#,
                params![id],
                |row| {
                    Ok(Picture {
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

impl Repository {
    pub async fn create_api_key_and_return_secret(
        &self,
        id: &str,
        scope: &str,
    ) -> anyhow::Result<String> {
        // Generate random 32-byte secret
        let mut raw = [0u8; 32];
        OsRng.try_fill_bytes(&mut raw)?;
        let secret = hex::encode(raw);

        self.create_api_key(id, scope, &secret).await?;
        Ok(secret)
    }

    pub async fn list_api_keys(&self) -> anyhow::Result<Vec<KeyInfo>> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            let mut stmt =
                conn.prepare("SELECT id, scope, created_at FROM api_keys ORDER BY created_at")?;
            let rows = stmt
                .query_map([], |r| {
                    Ok(KeyInfo {
                        id: r.get(0)?,
                        scope: r.get(1)?,
                        created: r.get(2)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(rows)
        })
        .await?
    }

    pub async fn delete_api_key(&self, id: &str) -> anyhow::Result<bool> {
        let pool = self.pool.clone();
        let id = id.to_owned();
        tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            let n = conn.execute("DELETE FROM api_keys WHERE id = ?1", params![id])?;
            Ok(n == 1)
        })
        .await?
    }
}

impl Repository {
    pub async fn create_api_key(&self, id: &str, scope: &str, secret: &str) -> Result<()> {
        let hash = Self::hash_secret(secret)?;
        let id = id.to_owned();
        let scope = scope.to_owned();
        let pool = self.pool.clone();
        task::spawn_blocking(move || {
            let conn = pool.get()?;
            conn.execute(
                "INSERT INTO api_keys (id, token_hash, scope, created_at)
                 VALUES (?1, ?2, ?3, strftime('%s','now'))",
                params![id, hash, scope],
            )?;
            Ok(())
        })
        .await?
    }

    pub async fn verify_api_key(
        &self,
        secret: &str,
    ) -> Result<Option<(String /*id*/, String /*scope*/)>> {
        let pool = self.pool.clone();
        let secret = secret.to_owned();
        task::spawn_blocking(move || {
            let conn = pool.get()?;

            // tiny table: fetch **all** hashes and compare locally
            let mut stmt = conn.prepare("SELECT id, token_hash, scope FROM api_keys")?;
            let rows = stmt.query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })?;

            for row in rows {
                let (id, hash, scope) = row?;
                if Self::verify_secret(&secret, &hash) {
                    return Ok(Some((id, scope)));
                }
            }
            Ok(None)
        })
        .await?
    }

    fn hash_secret(secret: &str) -> Result<String> {
        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(secret.as_bytes(), &salt)
            .map_err(|err| {
                tracing::warn!("Error hashing password: {:?}", err);
                anyhow::anyhow!("Error hashing password: {}", err)
            })?
            .to_string();
        Ok(hash)
    }

    fn verify_secret(secret: &str, hash: &str) -> bool {
        let parsed = match PasswordHash::new(hash) {
            Ok(p) => p,
            Err(_) => return false,
        };
        Argon2::default()
            .verify_password(secret.as_bytes(), &parsed)
            .is_ok()
    }
}

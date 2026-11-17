//! Session persistence and the session layer.
//!
//! The store is hand-rolled on the workspace sqlx 0.9 rather than taken from
//! `tower-sessions-sqlx-store` 0.15.1, which resolves against sqlx 0.8 and
//! would put two sqlx majors in one binary (ADR-0010). The table it reads is
//! the `tower_sessions.session` mirror of the migration set (ADR-0009).

use async_trait::async_trait;
use std::collections::HashMap;

use sqlx::PgPool;
use time::{Duration, OffsetDateTime};
use tower_sessions::session::{Id, Record};
use tower_sessions::session_store::{Error, Result, SessionStore};
use tower_sessions::{Expiry, SessionManagerLayer, cookie::SameSite};

use crate::state::AppState;

#[derive(Debug, Clone)]
pub struct PostgresSessionStore {
    pool: PgPool,
}

impl PostgresSessionStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SessionStore for PostgresSessionStore {
    async fn create(&self, record: &mut Record) -> Result<()> {
        let inserted = sqlx::query!(
            "INSERT INTO tower_sessions.session (id, data, expiry_date) VALUES ($1, $2, $3) ON CONFLICT (id) DO NOTHING",
            record.id.to_string(),
            encoded_data(&record.data)?,
            record.expiry_date,
        )
        .execute(&self.pool)
        .await
        .map_err(backend)?;
        if inserted.rows_affected() == 0 {
            return Err(Error::Backend("session id already taken".to_owned()));
        }
        Ok(())
    }

    async fn save(&self, record: &Record) -> Result<()> {
        let saved = sqlx::query!(
            "INSERT INTO tower_sessions.session (id, data, expiry_date) VALUES ($1, $2, $3) ON CONFLICT (id) DO UPDATE SET data = EXCLUDED.data, expiry_date = EXCLUDED.expiry_date",
            record.id.to_string(),
            encoded_data(&record.data)?,
            record.expiry_date,
        )
        .execute(&self.pool)
        .await
        .map_err(backend)?;
        assert_inserted(saved.rows_affected())
    }

    async fn load(&self, session_id: &Id) -> Result<Option<Record>> {
        let stored = sqlx::query!(
            "SELECT id, data, expiry_date FROM tower_sessions.session WHERE id = $1",
            session_id.to_string(),
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(backend)?;
        let Some(stored) = stored else {
            return Ok(None);
        };
        if stored.expiry_date < OffsetDateTime::now_utc() {
            return Ok(None);
        }
        Ok(Some(record_from_row(
            session_id,
            &stored.data,
            stored.expiry_date,
        )?))
    }

    async fn delete(&self, session_id: &Id) -> Result<()> {
        let deleted = sqlx::query!(
            "DELETE FROM tower_sessions.session WHERE id = $1",
            session_id.to_string(),
        )
        .execute(&self.pool)
        .await
        .map_err(backend)?;
        assert_inserted(deleted.rows_affected())
    }
}

fn encoded_data(data: &HashMap<String, serde_json::Value>) -> Result<Vec<u8>> {
    serde_json::to_vec(data).map_err(|source| Error::Encode(source.to_string()))
}

fn record_from_row(session_id: &Id, data: &[u8], expiry_date: OffsetDateTime) -> Result<Record> {
    Ok(Record {
        id: *session_id,
        data: serde_json::from_slice(data).map_err(|source| Error::Decode(source.to_string()))?,
        expiry_date,
    })
}

fn backend(source: sqlx::Error) -> Error {
    Error::Backend(source.to_string())
}

/// why: an upsert or delete that touches no row means the store and the layer
/// disagree about state; failing fast surfaces it instead of hiding it.
fn assert_inserted(rows: u64) -> Result<()> {
    if rows == 0 {
        return Err(Error::Backend("the store touched no rows".to_owned()));
    }
    Ok(())
}

/// The session layer: the configured cookie with the documented flags, a
/// thirty-day sliding window through `always_save`, and the mirror table as
/// the store. The twelve-hour absolute cap is enforced by the session
/// lifetime middleware (docs/10-security.md §1.4); tower-sessions has no
/// absolute-expiry variant.
pub fn session_layer(state: &AppState) -> SessionManagerLayer<PostgresSessionStore> {
    let session = &state.settings().session;
    SessionManagerLayer::new(PostgresSessionStore::new(state.pool().clone()))
        .with_name(session.cookie_name.clone())
        .with_secure(true)
        .with_http_only(true)
        .with_same_site(SameSite::Lax)
        .with_path("/")
        .with_expiry(Expiry::OnInactivity(Duration::seconds(i64::from(
            session.idle_seconds,
        ))))
        .with_always_save(true)
}

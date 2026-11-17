//! Ephemeral Postgres harness shared by every database-backed test.
//!
//! Each [`TestDatabase`] boots a private `initdb`-initialised Postgres on a
//! unix socket inside a temp directory, applies the workspace migrations and
//! hands out a pool. No system service, container or network port is involved,
//! so the suite runs identically on a developer machine and in CI.

pub mod ephemeral;
pub mod migration;

use std::sync::atomic::{AtomicU64, Ordering};

use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

pub use crate::ephemeral::EphemeralPostgres;
pub use crate::migration::{RevertError, migrate, revert_all};

/// A migrated, empty database on a private Postgres instance.
pub struct TestDatabase {
    server: EphemeralPostgres,
    pool: PgPool,
}

impl TestDatabase {
    /// Boots a fresh Postgres, creates a uniquely named database and migrates it.
    pub async fn new() -> Result<Self, HarnessError> {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let server = EphemeralPostgres::start().await?;
        let database = unique_database_name(COUNTER.fetch_add(1, Ordering::Relaxed));
        server.create_database(&database).await?;
        let pool = PgPoolOptions::new()
            .max_connections(4)
            .connect_with(server.database_options(&database))
            .await?;
        migrate(&pool).await?;
        Ok(Self { server, pool })
    }

    /// Connection pool for the migrated database.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

fn unique_database_name(sequence: u64) -> String {
    format!("fantatt_test_{}_{sequence}", std::process::id())
}

impl Drop for TestDatabase {
    fn drop(&mut self) {
        self.server.stop();
    }
}

/// Failure modes of the ephemeral harness.
#[derive(Debug, thiserror::Error)]
pub enum HarnessError {
    #[error("postgres binaries are not on PATH: run the suite inside `nix develop`")]
    MissingPostgres,

    #[error("could not spawn the postgres server process")]
    Spawn(#[source] std::io::Error),

    #[error("could not prepare the ephemeral data directory")]
    Workspace(std::io::Error),

    #[error("the ephemeral postgres never became ready")]
    NotReady,

    #[error("database setup failed")]
    Database(#[from] sqlx::Error),

    #[error("migration failed")]
    Migrate(#[from] sqlx::migrate::MigrateError),

    #[error("revert failed")]
    Revert(#[from] RevertError),
}

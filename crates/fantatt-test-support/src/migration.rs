//! Migration runner and manual revert used by the schema tests.
//!
//! The sqlx `Migrator` applies migrations but has no revert API, so the
//! rollback half of the G0.5 verification (migrate → rollback → migrate)
//! reads the matching `.down.sql` and deletes the recorded version itself.

use std::fs;
use std::path::PathBuf;

use sqlx::migrate::{MigrateError, Migrator};
use sqlx::{PgPool, raw_sql};

pub static MIGRATOR: Migrator = sqlx::migrate!("../../migrations");

/// Applies every pending migration.
pub async fn migrate(pool: &PgPool) -> Result<(), MigrateError> {
    MIGRATOR.run(pool).await
}

/// Reverts every applied migration, newest version first.
pub async fn revert_all(pool: &PgPool) -> Result<(), RevertError> {
    while let Some(version) = latest_version(pool).await? {
        revert_one(pool, version).await?;
    }
    Ok(())
}

/// Reverts a single migration version.
pub async fn revert_one(pool: &PgPool, version: i64) -> Result<(), RevertError> {
    let down_sql = fs::read_to_string(down_file_for(version)?).map_err(RevertError::File)?;
    let mut transaction = pool.begin().await?;
    // why: the script is a repo-committed migration file, not request data.
    raw_sql(sqlx::AssertSqlSafe(down_sql))
        .execute(&mut *transaction)
        .await?;
    sqlx::query!("DELETE FROM _sqlx_migrations WHERE version = $1", version)
        .execute(&mut *transaction)
        .await?;
    transaction.commit().await?;
    Ok(())
}

async fn latest_version(pool: &PgPool) -> Result<Option<i64>, RevertError> {
    let version: Option<i64> = sqlx::query_scalar!(
        "SELECT version FROM _sqlx_migrations WHERE success ORDER BY version DESC LIMIT 1"
    )
    .fetch_optional(pool)
    .await?;
    Ok(version)
}

fn migrations_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("migrations")
}

fn down_file_for(version: i64) -> Result<PathBuf, RevertError> {
    let prefix = format!("{version}_");
    let entry = fs::read_dir(migrations_dir())
        .map_err(RevertError::Directory)?
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .find(|name| name.starts_with(&prefix) && name.ends_with(".down.sql"))
        .ok_or_else(|| RevertError::MissingDownFile(version))?;
    Ok(migrations_dir().join(entry))
}

/// Failure modes of the manual revert path.
#[derive(Debug, thiserror::Error)]
pub enum RevertError {
    #[error("no revert file found for migration version {0}")]
    MissingDownFile(i64),

    #[error("could not read the migrations directory")]
    Directory(std::io::Error),

    #[error("could not read a revert file")]
    File(std::io::Error),

    #[error("revert query failed")]
    Query(#[from] sqlx::Error),
}

//! Process lifecycle of a private Postgres instance for one test.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use sqlx::postgres::PgConnectOptions;
use sqlx::{Connection, PgConnection};

use crate::HarnessError;

const READY_ATTEMPTS: u32 = 100;
const READY_PAUSE: Duration = Duration::from_millis(50);

/// A Postgres server process owned by one test, talking over a unix socket.
pub struct EphemeralPostgres {
    data_dir: PathBuf,
    socket_dir: PathBuf,
    server: Option<Child>,
}

impl EphemeralPostgres {
    /// Initialises and starts a private Postgres instance.
    pub async fn start() -> Result<Self, HarnessError> {
        let root = unique_root();
        let data_dir = root.join("data");
        let socket_dir = root.join("socket");
        let log_path = root.join("postgres.log");
        fs::create_dir_all(&socket_dir).map_err(HarnessError::Workspace)?;
        initialise(&data_dir)?;
        let server = spawn_server(&data_dir, &socket_dir, &log_path)?;
        let instance = Self {
            data_dir,
            socket_dir,
            server: Some(server),
        };
        instance.wait_until_ready().await?;
        Ok(instance)
    }

    /// Creates a database on the running instance.
    pub async fn create_database(&self, name: &str) -> Result<(), HarnessError> {
        let options = self.maintenance_options();
        let mut connection = PgConnection::connect_with(&options).await?;
        // why: the statement is DDL, so it cannot take a bind parameter; the
        // name is generated from the process id and a counter, never user input.
        let statement = sqlx::AssertSqlSafe(format!("CREATE DATABASE \"{name}\""));
        sqlx::query(statement).execute(&mut connection).await?;
        Ok(())
    }

    /// Connection URL for one of the instance's databases.
    pub fn database_url(&self, name: &str) -> String {
        format!(
            "postgres://postgres@/{name}?host={}",
            self.socket_dir.display()
        )
    }

    /// Connection options for one of the instance's databases.
    pub fn database_options(&self, name: &str) -> PgConnectOptions {
        let socket = self.socket_dir.display().to_string();
        PgConnectOptions::new()
            .username("postgres")
            .database(name)
            .host(socket.as_str())
    }

    /// Kills the server and removes the temp directory. Idempotent.
    pub fn stop(&mut self) {
        if let Some(mut server) = self.server.take() {
            let _ = server.kill();
            let _ = server.wait();
        }
        let _ = fs::remove_dir_all(self.root());
    }

    fn root(&self) -> PathBuf {
        self.data_dir
            .parent()
            .expect("data dir always has a parent")
            .to_path_buf()
    }

    async fn wait_until_ready(&self) -> Result<(), HarnessError> {
        let options = self.maintenance_options();
        for _ in 0..READY_ATTEMPTS {
            if PgConnection::connect_with(&options).await.is_ok() {
                return Ok(());
            }
            tokio::time::sleep(READY_PAUSE).await;
        }
        Err(HarnessError::NotReady)
    }

    fn maintenance_options(&self) -> PgConnectOptions {
        self.database_options("postgres")
    }
}

impl Drop for EphemeralPostgres {
    fn drop(&mut self) {
        self.stop();
    }
}

fn unique_root() -> PathBuf {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "fantatt-ephemeral-{}-{sequence}",
        std::process::id()
    ))
}

fn initialise(data_dir: &Path) -> Result<(), HarnessError> {
    let status = Command::new("initdb")
        .arg("-D")
        .arg(data_dir)
        .arg("-U")
        .arg("postgres")
        .arg("--auth=trust")
        .arg("--no-sync")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|error| map_spawn_error("initdb", error))?;
    if !status.success() {
        return Err(HarnessError::Spawn(std::io::Error::other(
            "initdb failed to initialise the ephemeral cluster",
        )));
    }
    Ok(())
}

fn spawn_server(
    data_dir: &Path,
    socket_dir: &Path,
    log_path: &Path,
) -> Result<Child, HarnessError> {
    let log = fs::File::create(log_path).map_err(HarnessError::Workspace)?;
    Command::new("postgres")
        .arg("-D")
        .arg(data_dir)
        .arg("-k")
        .arg(socket_dir)
        .arg("-c")
        .arg("listen_addresses=")
        .stdout(Stdio::from(log))
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| map_spawn_error("postgres", error))
}

fn map_spawn_error(tool: &str, error: std::io::Error) -> HarnessError {
    if error.kind() == std::io::ErrorKind::NotFound {
        return HarnessError::MissingPostgres;
    }
    HarnessError::Spawn(std::io::Error::other(format!(
        "spawning {tool} failed: {error}"
    )))
}

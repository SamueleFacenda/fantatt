//! The API binary: load configuration, build the pool, serve the shell.

use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use fantatt_config::loader::Loaded;
use fantatt_config::{cli::Cli, environment::Environment, loader::load};
use sqlx::postgres::{PgPool, PgPoolOptions};

use fantatt_api::routes;
use fantatt_api::state::AppState;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let loaded = load_settings()?;
    let pool = connect_pool(&loaded).await?;
    let state = AppState::build(&loaded, pool)?;
    serve(state, loaded.settings()).await
}

fn load_settings() -> Result<Loaded, Box<dyn std::error::Error>> {
    let arguments: Vec<String> = std::env::args().collect();
    let cli = Cli::parse(arguments.iter().map(String::as_str))?;
    Ok(load(&Environment::from_process(), &cli)?)
}

async fn connect_pool(loaded: &Loaded) -> Result<PgPool, Box<dyn std::error::Error>> {
    let database = &loaded.settings().database;
    let url = loaded
        .secret("database.url")
        .ok_or_else(|| missing_secret("database.url"))?;
    Ok(PgPoolOptions::new()
        .max_connections(database.max_connections)
        .min_connections(database.min_connections)
        .acquire_timeout(Duration::from_secs(database.connect_timeout_seconds.into()))
        .connect(url.expose())
        .await?)
}

async fn serve(
    state: AppState,
    settings: &fantatt_config::settings::Settings,
) -> Result<(), Box<dyn std::error::Error>> {
    let bind: IpAddr =
        settings
            .server
            .bind
            .parse()
            .map_err(|source: std::net::AddrParseError| {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, source.to_string())
            })?;
    let listener =
        tokio::net::TcpListener::bind(SocketAddr::new(bind, settings.server.port)).await?;
    axum::serve(
        listener,
        routes::build_router(state).into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}

fn missing_secret(key: &str) -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!("the secret {key} did not resolve"),
    )
}

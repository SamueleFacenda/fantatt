//! Shared state: the loaded settings and the database pool.

use std::sync::Arc;

use fantatt_config::settings::Settings;
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    settings: Arc<Settings>,
    pool: PgPool,
}

impl AppState {
    pub fn new(settings: Settings, pool: PgPool) -> Self {
        Self {
            settings: Arc::new(settings),
            pool,
        }
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn state_shares_one_settings_arc_across_clones() {
        let settings = Settings::default();
        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://postgres@127.0.0.1:1/unused")
            .expect("connect options parse without a live server");
        let state = AppState::new(settings, pool);
        assert!(std::ptr::eq(state.settings(), state.clone().settings()));
    }
}

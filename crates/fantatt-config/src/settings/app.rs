use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DeploymentEnvironment {
    Production,
    Staging,
    Development,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct App {
    pub name: String,
    pub environment: DeploymentEnvironment,
    pub locale: String,
    pub base_url: String,
}

impl App {
    /// Insecure endpoints (plain HTTP, loopback hosts) are only tolerable while
    /// the deployment is not serving the public internet.
    pub fn is_production(&self) -> bool {
        self.environment == DeploymentEnvironment::Production
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn environment_variants_use_kebab_case() {
        assert_eq!(
            serde_json::to_string(&DeploymentEnvironment::Production).unwrap(),
            "\"production\""
        );
        assert_eq!(
            serde_json::to_string(&DeploymentEnvironment::Development).unwrap(),
            "\"development\""
        );
    }

    #[test]
    fn production_flag_tracks_the_selected_environment() {
        let mut app = App {
            name: "Fantatt".to_owned(),
            environment: DeploymentEnvironment::Development,
            locale: "it".to_owned(),
            base_url: "http://127.0.0.1:8080".to_owned(),
        };
        assert!(!app.is_production());
        app.environment = DeploymentEnvironment::Production;
        assert!(app.is_production());
    }
}

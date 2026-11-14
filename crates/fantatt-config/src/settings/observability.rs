use serde::{Deserialize, Serialize};

use crate::secret::SecretRef;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LogFormat {
    Json,
    Pretty,
    Compact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Observability {
    pub otlp_endpoint: String,
    pub service_name: String,
    pub log_level: LogLevel,
    pub log_format: LogFormat,
    pub otlp_headers: SecretRef,
}

impl Observability {
    pub fn exports_telemetry(&self) -> bool {
        !self.otlp_endpoint.trim().is_empty()
    }

    /// Collectors that require auth are only addressed once the header
    /// reference resolves to a value, so the requirement is enforced by the
    /// secret resolution step and not by the settings alone.
    pub fn needs_auth_headers(&self) -> bool {
        self.exports_telemetry() && self.otlp_endpoint.starts_with("https://")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observability(endpoint: &str) -> Observability {
        Observability {
            otlp_endpoint: endpoint.to_owned(),
            service_name: "fantatt-api".to_owned(),
            log_level: LogLevel::Info,
            log_format: LogFormat::Json,
            otlp_headers: SecretRef::try_from(
                "env:FANTATT__OBSERVABILITY__OTLP_HEADERS".to_owned(),
            )
            .unwrap(),
        }
    }

    #[test]
    fn log_enums_use_lowercase_kebab_case() {
        assert_eq!(serde_json::to_string(&LogFormat::Json).unwrap(), "\"json\"");
        assert_eq!(serde_json::to_string(&LogLevel::Warn).unwrap(), "\"warn\"");
    }

    #[test]
    fn an_empty_endpoint_means_telemetry_is_off() {
        assert!(!observability("").exports_telemetry());
        assert!(!observability("").needs_auth_headers());
    }

    #[test]
    fn a_remote_collector_needs_auth_headers() {
        assert!(observability("https://otel.example.org").needs_auth_headers());
        assert!(!observability("http://127.0.0.1:4317").needs_auth_headers());
    }
}

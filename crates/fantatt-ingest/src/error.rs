//! Error taxonomy for the fetch facade: every failure names the policy, the
//! circuit, the transport, or the status that produced it.

use crate::settings::SettingsViolation;

/// Why a fetch could not deliver a page.
#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    #[error("the host policy rejected {url}: {reason}")]
    Blocked { url: String, reason: String },

    #[error("the circuit breaker for {host} is open; the source keeps failing")]
    CircuitOpen { host: String },

    #[error("the request to {url} timed out after {timeout_ms} ms")]
    Timeout { url: String, timeout_ms: u64 },

    #[error("cannot reach {url}: {reason}")]
    Connect { url: String, reason: String },

    #[error("the transport refused {url}: {reason}")]
    Refused { url: String, reason: String },

    #[error("{url} answered {status}")]
    Status { url: String, status: u16 },

    #[error("{url} kept failing after {attempts} attempts: {reason}")]
    RetriesExhausted {
        url: String,
        attempts: u32,
        reason: String,
    },

    #[error("cannot store the response body under {key}: {reason}")]
    Archive { key: String, reason: String },

    #[error(transparent)]
    Settings(#[from] SettingsViolation),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_settings_violation_is_transparent() {
        let error = FetchError::from(SettingsViolation::Concurrency { value: 0 });
        assert!(matches!(error, FetchError::Settings(_)), "{error}");
        assert!(error.to_string().contains("concurrency"));
    }
}

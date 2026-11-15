use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Server {
    pub bind: String,
    pub port: u16,
    pub public_port: u16,
    pub request_body_limit_bytes: u64,
}

impl Server {
    /// `__Host-` cookies are rejected by browsers unless the page is served
    /// from the standard TLS port, so the pair is validated at startup.
    pub fn serves_tls_standard_port(&self) -> bool {
        self.public_port == 443
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn server(port: u16, public_port: u16) -> Server {
        Server {
            bind: "127.0.0.1".to_owned(),
            port,
            public_port,
            request_body_limit_bytes: 1_048_576,
        }
    }

    #[test]
    fn standard_tls_port_is_recognised() {
        assert!(server(8080, 443).serves_tls_standard_port());
        assert!(!server(8080, 8443).serves_tls_standard_port());
    }

    #[test]
    fn out_of_range_ports_are_rejected_by_the_type() {
        let json = r#"{"bind":"127.0.0.1","port":65536,"public_port":443,"request_body_limit_bytes":1048576}"#;
        assert!(serde_json::from_str::<Server>(json).is_err());
    }
}

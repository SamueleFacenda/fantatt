use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Live {
    pub sse_enabled: bool,
    pub heartbeat_seconds: u32,
}

impl Live {
    pub fn has_a_live_heartbeat(&self) -> bool {
        !self.sse_enabled || self.heartbeat_seconds >= 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_live_stream_needs_a_nonzero_heartbeat() {
        let mut live = Live {
            sse_enabled: true,
            heartbeat_seconds: 30,
        };
        assert!(live.has_a_live_heartbeat());
        live.heartbeat_seconds = 0;
        assert!(!live.has_a_live_heartbeat());
    }

    #[test]
    fn a_disabled_stream_tolerates_zero() {
        let live = Live {
            sse_enabled: false,
            heartbeat_seconds: 0,
        };
        assert!(live.has_a_live_heartbeat());
    }
}

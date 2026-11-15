use serde::{Deserialize, Serialize};

const HOST_COOKIE_PREFIX: &str = "__Host-";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    pub idle_seconds: u32,
    pub absolute_seconds: u32,
    pub cookie_name: String,
}

impl Session {
    pub fn uses_host_prefixed_cookie(&self) -> bool {
        self.cookie_name.starts_with(HOST_COOKIE_PREFIX)
    }

    /// A zero window would evict the session immediately, so neither may be zero.
    pub fn lifetimes_are_ordered(&self) -> bool {
        self.idle_seconds > 0 && self.absolute_seconds > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(cookie_name: &str) -> Session {
        Session {
            idle_seconds: 2_592_000,
            absolute_seconds: 43_200,
            cookie_name: cookie_name.to_owned(),
        }
    }

    #[test]
    fn host_prefix_is_detected() {
        assert!(session("__Host-session").uses_host_prefixed_cookie());
        assert!(!session("session").uses_host_prefixed_cookie());
    }

    #[test]
    fn lifetimes_must_be_positive() {
        assert!(session("__Host-session").lifetimes_are_ordered());
        let mut no_absolute = session("__Host-session");
        no_absolute.absolute_seconds = 0;
        assert!(!no_absolute.lifetimes_are_ordered());
        let mut no_idle = session("__Host-session");
        no_idle.idle_seconds = 0;
        assert!(!no_idle.lifetimes_are_ordered());
    }
}

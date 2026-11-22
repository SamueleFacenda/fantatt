//! Host allow-list and redirect policy.
//!
//! The portal is the only legitimate source (docs/06-data-pipeline.md §1), so
//! every request target and every redirect target must name an allowed host,
//! and a redirect keeps the origin of the request it follows
//! (docs/10-security.md §9).

use std::collections::HashSet;

use reqwest::Url;
use reqwest::redirect::Policy;

use crate::error::FetchError;

/// The hosts the facade may talk to.
#[derive(Debug, Clone, Default)]
pub struct HostPolicy {
    allowed: HashSet<String>,
}

impl HostPolicy {
    pub fn new(hosts: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            allowed: hosts
                .into_iter()
                .map(|host| host.into().to_ascii_lowercase())
                .collect(),
        }
    }

    pub fn allows(&self, host: &str) -> bool {
        self.allowed.contains(&host.to_ascii_lowercase())
    }

    /// Rejects a URL whose host the allow-list does not name.
    pub fn check(&self, url: &Url) -> Result<(), FetchError> {
        let host = host_of(url).ok_or_else(|| FetchError::Blocked {
            url: url.to_string(),
            reason: "the url names no host".to_owned(),
        })?;
        if self.allows(host) {
            return Ok(());
        }
        Err(FetchError::Blocked {
            url: url.to_string(),
            reason: format!("{host} is not an allowed host"),
        })
    }

    /// The reqwest redirect policy: followed only inside the allow-list and
    /// only back to the origin of the request that produced the redirect.
    pub fn redirect_policy(&self, max_redirects: usize) -> Policy {
        let allowed = self.clone();
        Policy::custom(move |attempt| {
            let redirect = Redirect {
                chain: attempt.previous(),
                target: attempt.url(),
            };
            match redirect_violation(&allowed, &redirect, max_redirects) {
                Some(reason) => attempt.error(reason),
                None => attempt.follow(),
            }
        })
    }
}

/// The host of a URL, if it names one.
pub fn host_of(url: &Url) -> Option<&str> {
    url.host_str()
}

/// What one redirect carries: the chain it arrives from and where it points.
pub(crate) struct Redirect<'a> {
    pub(crate) chain: &'a [Url],
    pub(crate) target: &'a Url,
}

/// What a redirect target violates, if anything: an unlisted host, a chain
/// longer than the ceiling, or an origin the target leaves.
fn redirect_violation(
    allowed: &HostPolicy,
    redirect: &Redirect<'_>,
    max_redirects: usize,
) -> Option<String> {
    let Some(host) = host_of(redirect.target) else {
        return Some("the redirect target names no host".to_owned());
    };
    if !allowed.allows(host) {
        return Some(format!("{host} is not an allowed host"));
    }
    if redirect.chain.len() > max_redirects {
        return Some(format!("more than {max_redirects} redirects"));
    }
    let origin = redirect.chain.first()?;
    if host_of(origin) != Some(host) || origin.scheme() != redirect.target.scheme() {
        return Some(format!(
            "the redirect leaves {}://{} to {}://{host}",
            origin.scheme(),
            host_of(origin).unwrap_or_default(),
            redirect.target.scheme()
        ));
    }
    (origin.port_or_known_default() != redirect.target.port_or_known_default())
        .then(|| "the redirect changes the port".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> HostPolicy {
        HostPolicy::new(["portale.fitet.org", "127.0.0.1"])
    }

    /// A redirect from one origin to one target, the shape of the unit cases.
    fn redirect_of<'a>(origin: &'a Url, target: &'a Url) -> Redirect<'a> {
        Redirect {
            chain: std::slice::from_ref(origin),
            target,
        }
    }

    #[test]
    fn an_allowed_host_passes_and_an_unknown_one_is_blocked() {
        let url = Url::parse("https://portale.fitet.org/menu.php").expect("url");
        assert!(policy().check(&url).is_ok());
        let other = Url::parse("https://example.org/menu.php").expect("url");
        let error = policy().check(&other).expect_err("blocked");
        assert!(
            matches!(&error, FetchError::Blocked { reason, .. } if reason.contains("example.org")),
            "{error}"
        );
    }

    #[test]
    fn the_host_match_ignores_the_case() {
        assert!(policy().allows("PORTALE.FITET.ORG"));
        assert!(!policy().allows("www.example.org"));
    }

    #[test]
    fn a_same_origin_redirect_is_followed() {
        let origin = Url::parse("http://127.0.0.1:8080/loop").expect("origin");
        let target = Url::parse("http://127.0.0.1:8080/destination").expect("target");
        assert_eq!(
            redirect_violation(&policy(), &redirect_of(&origin, &target), 5),
            None
        );
    }

    #[test]
    fn a_redirect_to_another_host_is_rejected() {
        let origin = Url::parse("http://127.0.0.1:8080/loop").expect("origin");
        let target = Url::parse("http://127.0.0.2:8080/destination").expect("target");
        let violation = redirect_violation(&policy(), &redirect_of(&origin, &target), 5);
        assert!(
            matches!(&violation, Some(reason) if reason.contains("not an allowed host")),
            "{violation:?}"
        );
    }

    #[test]
    fn a_scheme_downgrade_is_rejected() {
        let origin = Url::parse("https://portale.fitet.org/menu.php").expect("origin");
        let target = Url::parse("http://portale.fitet.org/menu.php").expect("target");
        let violation = redirect_violation(&policy(), &redirect_of(&origin, &target), 5);
        assert!(
            matches!(&violation, Some(reason) if reason.contains("leaves")),
            "{violation:?}"
        );
    }

    #[test]
    fn a_port_change_is_rejected() {
        let origin = Url::parse("http://127.0.0.1:8080/loop").expect("origin");
        let target = Url::parse("http://127.0.0.1:9090/destination").expect("target");
        let violation = redirect_violation(&policy(), &redirect_of(&origin, &target), 5);
        assert!(
            matches!(&violation, Some(reason) if reason.contains("port")),
            "{violation:?}"
        );
    }

    fn hops(count: usize) -> Vec<Url> {
        (0..count)
            .map(|index| Url::parse(&format!("http://127.0.0.1:8080/hop{index}")).expect("url"))
            .collect()
    }

    #[test]
    fn a_long_chain_is_rejected() {
        let origin = Url::parse("http://127.0.0.1:8080/loop").expect("origin");
        let target = Url::parse("http://127.0.0.1:8080/destination").expect("target");
        let short = redirect_of(&origin, &target);
        let chain = hops(6);
        let redirect = Redirect {
            chain: &chain,
            target: &target,
        };
        assert_eq!(redirect_violation(&policy(), &short, 5), None);
        let violation = redirect_violation(&policy(), &redirect, 5);
        assert!(
            matches!(&violation, Some(reason) if reason.contains("more than 5")),
            "{violation:?}"
        );
    }
}

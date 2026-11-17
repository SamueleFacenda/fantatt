//! Rate limits: the budgets of docs/07-api-spec.md §14 parsed from
//! configuration, enforced per key by governor.

use std::collections::BTreeMap;
use std::num::NonZeroU32;
use std::time::Duration;

use axum::extract::{Request, State};
use axum::http::{HeaderValue, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use governor::clock::{Clock, DefaultClock};
use governor::{DefaultKeyedRateLimiter, NotUntil, Quota, RateLimiter};

use crate::error::{Problem, ProblemType};
use crate::state::AppState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitKey {
    Ip,
    Session,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RateRule {
    pub requests: u32,
    pub window_seconds: u32,
    pub key: LimitKey,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RateRuleError {
    #[error("a rate rule reads '<count>/<unit> per <key>', as in '10/min per ip'")]
    MalformedRule,
    #[error("a rate rule needs a positive request count")]
    ZeroRequests,
    #[error("'{unit}' is not a time unit (sec, min, hour, day)")]
    UnknownUnit { unit: String },
    #[error("'{key}' is not a limit key (ip, session, user, account, manager)")]
    UnknownKey { key: String },
}

pub fn parse_rule(text: &str) -> Result<RateRule, RateRuleError> {
    let mut parts = text.split_whitespace();
    let (Some(budget), Some(_per), Some(key)) = (parts.next(), parts.next(), parts.next()) else {
        return Err(RateRuleError::MalformedRule);
    };
    if parts.next().is_some() {
        return Err(RateRuleError::MalformedRule);
    }
    let (requests, unit) = budget.split_once('/').ok_or(RateRuleError::MalformedRule)?;
    let requests: u32 = requests.parse().map_err(|_| RateRuleError::MalformedRule)?;
    if requests == 0 {
        return Err(RateRuleError::ZeroRequests);
    }
    Ok(RateRule {
        requests,
        window_seconds: window_seconds(unit)?,
        key: limit_key(key)?,
    })
}

fn window_seconds(unit: &str) -> Result<u32, RateRuleError> {
    match unit {
        "sec" => Ok(1),
        "min" => Ok(60),
        "hour" => Ok(3_600),
        "day" => Ok(86_400),
        _ => Err(RateRuleError::UnknownUnit {
            unit: unit.to_owned(),
        }),
    }
}

fn limit_key(key: &str) -> Result<LimitKey, RateRuleError> {
    match key {
        "ip" => Ok(LimitKey::Ip),
        "session" | "user" | "account" | "manager" => Ok(LimitKey::Session),
        _ => Err(RateRuleError::UnknownKey {
            key: key.to_owned(),
        }),
    }
}

pub type KeyedLimiter = DefaultKeyedRateLimiter<String>;

/// The rejection a keyed limiter reports when the budget is exhausted.
type LimitNotUntil = NotUntil<<DefaultClock as Clock>::Instant>;

/// The parsed rules, each holding its own keyed limiter.
#[derive(Debug)]
pub struct RateLimits {
    limiters: BTreeMap<String, (RateRule, KeyedLimiter)>,
}

impl RateLimits {
    pub fn build(rules: &BTreeMap<String, String>) -> Result<Self, RateRuleError> {
        let mut limiters = BTreeMap::new();
        for (name, text) in rules {
            let rule = parse_rule(text)?;
            let quota = Quota::with_period(Duration::from_secs(rule.window_seconds.into()))
                .and_then(|quota| Some(quota.allow_burst(NonZeroU32::new(rule.requests)?)));
            let Some(quota) = quota else {
                return Err(RateRuleError::ZeroRequests);
            };
            limiters.insert(name.clone(), (rule, RateLimiter::keyed(quota)));
        }
        Ok(Self { limiters })
    }

    pub fn rule(&self, name: &str) -> Option<RateRule> {
        self.limiters.get(name).map(|(rule, _)| rule.clone())
    }

    fn check(&self, name: &str, key: &str) -> Option<LimitNotUntil> {
        let (_, limiter) = self.limiters.get(name)?;
        limiter.check_key(&key.to_owned()).err()
    }
}

pub fn rules_for_path(path: &str, query: Option<&str>) -> &'static [&'static str] {
    if path.starts_with("/auth/") {
        &["auth", "auth-account"]
    } else if path.starts_with("/players") && query.is_some_and(|q| q.contains("search=")) {
        &["search"]
    } else {
        &["reads"]
    }
}

pub async fn enforce_rate_limits(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let path = request.uri().path().to_owned();
    let query = request.uri().query().map(str::to_owned);
    let session_id = session_key(&request);
    let peer_ip = peer_ip(&request);
    for name in rules_for_path(&path, query.as_deref()) {
        let Some(rule) = state.rate_limits().rule(name) else {
            continue;
        };
        // why: a session-scoped budget needs an identity to key on; binding
        // every anonymous visitor to one bucket would let a single client
        // exhaust the limit for all of them.
        if rule.key == LimitKey::Session && session_id.is_none() {
            continue;
        }
        let key = limit_key_for(rule.key, &session_id, &peer_ip);
        if let Some(not_until) = state.rate_limits().check(name, &key) {
            return too_many_requests(not_until);
        }
    }
    next.run(request).await
}

fn session_key(request: &Request) -> Option<String> {
    request
        .extensions()
        .get::<tower_sessions::Session>()
        .and_then(|session| session.id())
        .map(|id| id.to_string())
}

fn peer_ip(request: &Request) -> Option<String> {
    request
        .extensions()
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
        .map(|connect_info| connect_info.0.ip().to_string())
}

fn limit_key_for(key: LimitKey, session_id: &Option<String>, peer_ip: &Option<String>) -> String {
    match key {
        LimitKey::Ip => format!("ip:{}", peer_ip.as_deref().unwrap_or("local")),
        LimitKey::Session => format!("session:{}", session_id.as_deref().unwrap_or("anonymous")),
    }
}

fn too_many_requests(not_until: LimitNotUntil) -> Response {
    let problem = Problem::from_type(ProblemType::TooManyRequests).with_code("E_RateLimited");
    let mut response = problem.into_response();
    let retry_after = not_until
        .wait_time_from(DefaultClock::default().now())
        .as_secs()
        .saturating_add(1);
    response.headers_mut().insert(
        header::RETRY_AFTER,
        HeaderValue::from_str(&retry_after.to_string()).expect("a number is a valid header"),
    );
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    const MALFORMED_RULES: [&str; 5] = [
        "",
        "10/min",
        "ten/min per ip",
        "-1/min per ip",
        "10/min per ip extra",
    ];

    fn rules() -> BTreeMap<String, String> {
        [
            ("auth", "10/min per ip"),
            ("auth-account", "5/min per account"),
            ("bid", "20/min per manager"),
            ("search", "60/min per user"),
            ("reads", "300/min per session"),
        ]
        .map(|(name, text)| (name.to_owned(), text.to_owned()))
        .into()
    }

    #[test]
    fn every_documented_rule_parses() {
        for (name, text) in rules() {
            let rule = parse_rule(&text).expect("a documented rule parses");
            match name.as_str() {
                "auth" => {
                    assert_eq!(rule.requests, 10);
                    assert_eq!(rule.window_seconds, 60);
                    assert_eq!(rule.key, LimitKey::Ip);
                }
                "auth-account" => assert_eq!(rule.requests, 5),
                "search" => assert_eq!(rule.requests, 60),
                "reads" => assert_eq!(rule.key, LimitKey::Session),
                _ => assert_eq!(rule.requests, 20),
            }
        }
    }

    #[test]
    fn a_malformed_rule_names_its_problem() {
        for text in MALFORMED_RULES {
            assert_eq!(
                parse_rule(text),
                Err(RateRuleError::MalformedRule),
                "{text}"
            );
        }
        assert_eq!(parse_rule("0/min per ip"), Err(RateRuleError::ZeroRequests));
        assert!(matches!(
            parse_rule("10/week per ip"),
            Err(RateRuleError::UnknownUnit { unit }) if unit == "week"
        ));
        assert!(matches!(
            parse_rule("10/min per moon"),
            Err(RateRuleError::UnknownKey { key }) if key == "moon"
        ));
    }

    #[test]
    fn an_absent_key_falls_back_to_the_local_bucket() {
        let rule = parse_rule("10/min per ip").expect("parse");
        assert_eq!(limit_key_for(rule.key, &None, &None), "ip:local");
        let session_rule = parse_rule("300/min per session").expect("parse");
        assert_eq!(
            limit_key_for(session_rule.key, &None, &None),
            "session:anonymous"
        );
        assert_eq!(
            limit_key_for(session_rule.key, &Some("abc".to_owned()), &None),
            "session:abc"
        );
    }

    #[test]
    fn a_budget_exhausts_once_per_key() {
        let limits = RateLimits::build(&rules()).expect("build");
        for _ in 0..10 {
            assert!(limits.check("auth", "ip:1").is_none());
        }
        assert!(limits.check("auth", "ip:1").is_some());
        assert!(limits.check("auth", "ip:2").is_none());
    }

    #[test]
    fn an_unknown_rule_is_ignored() {
        let limits = RateLimits::build(&rules()).expect("build");
        assert!(limits.check("teleport", "ip:1").is_none());
    }

    #[test]
    fn documented_paths_map_to_budgets() {
        assert_eq!(
            rules_for_path("/auth/csrf", None),
            &["auth", "auth-account"]
        );
        assert_eq!(
            rules_for_path("/auth/session", None),
            &["auth", "auth-account"]
        );
        assert_eq!(
            rules_for_path("/players", Some("search=mario")),
            &["search"]
        );
        assert_eq!(rules_for_path("/players", Some("tier=c1")), &["reads"]);
        assert_eq!(rules_for_path("/healthz", None), &["reads"]);
        assert_eq!(rules_for_path("/openapi.json", None), &["reads"]);
        assert_eq!(rules_for_path("/api/v1/csp-report", None), &["reads"]);
        assert_eq!(rules_for_path("/leagues", None), &["reads"]);
    }
}

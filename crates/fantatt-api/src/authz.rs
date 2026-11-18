//! Route-level authorization: declared capabilities, the default-deny guard,
//! and the policy of docs/10-security.md §4.
//!
//! Every route enters through a declaration naming its capability and the
//! object it acts on, so a route registered without a declaration is denied
//! before it reaches a handler.

use std::str::FromStr;
use std::sync::Arc;

use axum::Router;
use axum::extract::{MatchedPath, Request, State};
use axum::handler::Handler;
use axum::http::Method;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::MethodRouter;
use sqlx::PgPool;
use tower_sessions::Session;

use fantatt_core::authz as policy;
use fantatt_core::authz::{
    Capability, Decision, Identifier, LeagueRole, ObjectScope, Principal, Role,
};

use crate::error::{Problem, ProblemType};
use crate::state::AppState;

/// The session key holding the authenticated user id.
pub const PRINCIPAL_ID_KEY: &str = "principal_id";

/// The object a guard is exercised against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectRef {
    Platform,
    League { param: &'static str },
    Account { param: &'static str },
}

/// A platform-scoped guard object.
pub fn platform() -> ObjectRef {
    ObjectRef::Platform
}

/// A league-scoped guard object, resolved from the named path parameter.
pub fn league(param: &'static str) -> ObjectRef {
    ObjectRef::League { param }
}

/// An account-scoped guard object, resolved from the named path parameter.
pub fn account(param: &'static str) -> ObjectRef {
    ObjectRef::Account { param }
}

/// The capability a route requires, and the object it requires it against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Guard {
    capability: Capability,
    object: ObjectRef,
}

/// Declares the capability a route serves under.
pub fn guard(capability: Capability, object: ObjectRef) -> Guard {
    Guard { capability, object }
}

/// One declared route: its method, its handler router, and its guard.
pub struct DeclaredRoute<S> {
    method: Method,
    method_router: MethodRouter<S>,
    guard: Guard,
}

impl<S> DeclaredRoute<S> {
    pub fn new(method: Method, method_router: MethodRouter<S>, guard: Guard) -> Self {
        Self {
            method,
            method_router,
            guard,
        }
    }
}

/// Declares a `GET` route under a guard.
pub fn get<H, T, S>(handler: H, guard: Guard) -> DeclaredRoute<S>
where
    H: Handler<T, S>,
    T: 'static,
    S: Clone + Send + Sync + 'static,
{
    DeclaredRoute::new(Method::GET, axum::routing::get(handler), guard)
}

/// Declares a `POST` route under a guard.
pub fn post<H, T, S>(handler: H, guard: Guard) -> DeclaredRoute<S>
where
    H: Handler<T, S>,
    T: 'static,
    S: Clone + Send + Sync + 'static,
{
    DeclaredRoute::new(Method::POST, axum::routing::post(handler), guard)
}

/// Declares a `PUT` route under a guard.
pub fn put<H, T, S>(handler: H, guard: Guard) -> DeclaredRoute<S>
where
    H: Handler<T, S>,
    T: 'static,
    S: Clone + Send + Sync + 'static,
{
    DeclaredRoute::new(Method::PUT, axum::routing::put(handler), guard)
}

/// Declares a `DELETE` route under a guard.
pub fn delete<H, T, S>(handler: H, guard: Guard) -> DeclaredRoute<S>
where
    H: Handler<T, S>,
    T: 'static,
    S: Clone + Send + Sync + 'static,
{
    DeclaredRoute::new(Method::DELETE, axum::routing::delete(handler), guard)
}

/// The declaration recorded for one route.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteDeclaration {
    method: Method,
    path: &'static str,
    guard: Guard,
}

/// Every declared route, the table the authorize middleware checks against.
#[derive(Debug, Clone, Default)]
pub struct RouteTable(Vec<RouteDeclaration>);

impl RouteTable {
    /// The declaration matching a method and matched path, if any.
    pub fn lookup(&self, method: &Method, path: &str) -> Option<&RouteDeclaration> {
        self.0
            .iter()
            .find(|declaration| declaration.method == *method && declaration.path == path)
    }
}

/// A router that cannot register a route without declaring its guard.
pub struct DeclaredRouter<S> {
    router: Router<S>,
    table: RouteTable,
}

impl<S> DeclaredRouter<S>
where
    S: Clone + Send + Sync + 'static,
{
    pub fn new() -> Self {
        Self {
            router: Router::new(),
            table: RouteTable::default(),
        }
    }

    /// Registers a declared route; the guard arrives with the route itself, so
    /// a registration without a capability does not compile.
    pub fn route(mut self, path: &'static str, route: DeclaredRoute<S>) -> Self {
        self.router = self.router.route(path, route.method_router);
        self.table.0.push(RouteDeclaration {
            method: route.method,
            path,
            guard: route.guard,
        });
        self
    }

    /// The axum router beside its declaration table.
    pub fn build(self) -> (Router<S>, RouteTable) {
        (self.router, self.table)
    }
}

impl<S> Default for DeclaredRouter<S>
where
    S: Clone + Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

enum Verdict {
    Allow,
    Deny(Problem),
}

/// Denies a request whose route declares no capability, or declares one the
/// session principal does not hold against the route's object.
pub async fn enforce_authorization(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let Some(table) = request.extensions().get::<Arc<RouteTable>>().cloned() else {
        return next.run(request).await;
    };
    match verdict(state, table, request).await {
        (Verdict::Allow, request) => next.run(request).await,
        (Verdict::Deny(problem), _request) => {
            let code = problem
                .extras
                .as_ref()
                .and_then(|extras| extras.code.clone());
            tracing::warn!(code = ?code, "authorization denied the request");
            problem.into_response()
        }
    }
}

async fn verdict(state: AppState, table: Arc<RouteTable>, request: Request) -> (Verdict, Request) {
    let Some(matched) = matched_path(&request) else {
        return (Verdict::Allow, request);
    };
    let Some(guard) = guard_for(&table, request.method(), matched.as_str()) else {
        return (Verdict::Deny(undeclared()), request);
    };
    let session_id =
        session_principal_id(crate::middleware::session_lifetime::live_session(&request)).await;
    let scope = object_scope(&guard.object, request.uri().path(), matched.as_str());
    let decision = decide(state, guard, (session_id, scope)).await;
    let verdict = if decision == Decision::Allow {
        Verdict::Allow
    } else {
        Verdict::Deny(denied(guard.capability))
    };
    (verdict, request)
}

fn matched_path(request: &Request) -> Option<MatchedPath> {
    request.extensions().get::<MatchedPath>().cloned()
}

fn guard_for(table: &RouteTable, method: &Method, pattern: &str) -> Option<Guard> {
    table
        .lookup(method, pattern)
        .map(|declaration| declaration.guard)
}

/// why: owned arguments keep the middleware future `Send` and `'static`, which
/// the from_fn layer bounds require.
async fn session_principal_id(session: Option<Session>) -> Option<String> {
    let session = session?;
    let principal_id: Option<String> = session.get(PRINCIPAL_ID_KEY).await.unwrap_or(None);
    principal_id
}

async fn decide(
    state: AppState,
    guard: Guard,
    caller: (Option<String>, Option<ObjectScope>),
) -> Decision {
    if guard.capability == Capability::Public {
        return Decision::Allow;
    }
    let (Some(session_id), Some(scope)) = caller else {
        return Decision::Deny;
    };
    let principal = resolve_principal(state, session_id, scope.clone()).await;
    if policy::authorize(guard.capability, &scope, principal.as_ref()) == Decision::Allow {
        Decision::Allow
    } else {
        Decision::Deny
    }
}

async fn resolve_principal(
    state: AppState,
    user_id: String,
    scope: ObjectScope,
) -> Option<Principal> {
    let mut superadmin = state.settings().authz.is_superadmin(&user_id);
    let mut league_roles = Vec::new();
    if let ObjectScope::League(league_id) = &scope {
        let stored_role =
            stored_league_role(state.pool().clone(), user_id.clone(), league_id.clone()).await;
        if stored_role == Some(Role::Superadmin) {
            superadmin = true;
        } else if let Some(role) = stored_role {
            league_roles.push(LeagueRole::new(league_id.clone(), role));
        }
    }
    let identifier = Identifier::from_str(&user_id).ok()?;
    Some(Principal::new(identifier, superadmin, league_roles))
}

/// why: a runtime query keeps the declaration table independent of the
/// offline query cache, which only code-checked macros regenerate.
async fn stored_league_role(pool: PgPool, user_id: String, league_id: Identifier) -> Option<Role> {
    let record = sqlx::query_as::<_, (String,)>(
        "SELECT role FROM fantatt.memberships
         WHERE league_id = $1 AND user_id = $2 AND state = 'ACTIVE'",
    )
    .bind(league_id.to_string())
    .bind(user_id)
    .fetch_optional(&pool)
    .await
    .ok()?;
    Role::from_db_str(&record?.0)
}

fn object_scope(object: &ObjectRef, path: &str, pattern: &str) -> Option<ObjectScope> {
    match object {
        ObjectRef::Platform => Some(ObjectScope::Platform),
        ObjectRef::League { param } => {
            let league_id = Identifier::from_str(&path_param(pattern, path, param)?).ok()?;
            Some(ObjectScope::League(league_id))
        }
        ObjectRef::Account { param } => {
            let account_id = Identifier::from_str(&path_param(pattern, path, param)?).ok()?;
            Some(ObjectScope::Account(account_id))
        }
    }
}

/// why: axum matches `MatchedPath` as a pattern while the concrete values ride
/// in the request path, so scope resolution zips the two.
pub fn path_param(pattern: &str, path: &str, param: &str) -> Option<String> {
    let mut patterns = pattern.split('/');
    let mut values = path.split('/');
    loop {
        match (patterns.next(), values.next()) {
            (Some(expected), Some(value)) => {
                if expected == format!("{{{param}}}") {
                    return Some(value.to_owned());
                }
                if expected != value {
                    return None;
                }
            }
            _ => return None,
        }
    }
}

fn denied(capability: Capability) -> Problem {
    Problem::from_type(ProblemType::CapabilityMissing)
        .with_code("CAPABILITY_MISSING")
        .with_details(serde_json::json!({ "capability": capability.name() }))
}

fn undeclared() -> Problem {
    Problem::from_type(ProblemType::CapabilityMissing).with_code("ROUTE_UNDECLARED")
}

#[cfg(test)]
mod tests {
    use super::*;

    const LEAGUE_ID: &str = "lg_01ARZ3XK7M0C4V6B8N2QWE9RTA";

    #[test]
    fn path_params_resolve_pattern_placeholders() {
        let pattern = "/leagues/{league_id}";
        let path = format!("/leagues/{LEAGUE_ID}");
        assert_eq!(
            path_param(pattern, &path, "league_id").as_deref(),
            Some(LEAGUE_ID)
        );
        assert_eq!(path_param(pattern, &path, "squad_id"), None);
        assert_eq!(path_param("/healthz", "/healthz", "league_id"), None);
        assert_eq!(
            path_param("/leagues/{league_id}", "/leagues", "league_id"),
            None
        );
        assert_eq!(
            path_param("/leagues/{league_id}/squads/{squad_id}", &path, "squad_id"),
            None
        );
    }

    #[test]
    fn object_refs_resolve_scopes_from_path_params() {
        let pattern = "/leagues/{league_id}";
        let path = format!("/leagues/{LEAGUE_ID}");
        let scope = object_scope(&league("league_id"), &path, pattern).expect("league scope");
        assert_eq!(
            scope,
            ObjectScope::League(Identifier::from_str(LEAGUE_ID).expect("id"))
        );
        assert_eq!(
            object_scope(&platform(), &path, pattern),
            Some(ObjectScope::Platform)
        );
        assert_eq!(object_scope(&league("squad_id"), &path, pattern), None);
    }

    #[test]
    fn a_declared_table_lookup_matches_method_and_path() {
        let table = RouteTable(vec![RouteDeclaration {
            method: Method::GET,
            path: "/healthz",
            guard: guard(Capability::Public, platform()),
        }]);
        assert!(table.lookup(&Method::GET, "/healthz").is_some());
        assert!(table.lookup(&Method::POST, "/healthz").is_none());
        assert!(table.lookup(&Method::GET, "/openapi.json").is_none());
    }

    #[test]
    fn a_denial_names_the_missing_capability() {
        let problem = denied(Capability::LeagueRead);
        assert_eq!(problem.status, 403);
        assert_eq!(
            problem.type_uri,
            "https://fantatt.org/problems/capability-missing"
        );
        let extras = problem.extras.expect("extras");
        assert_eq!(extras.code.as_deref(), Some("CAPABILITY_MISSING"));
        assert_eq!(
            extras.details.expect("details")["capability"],
            "league:read"
        );
    }
}

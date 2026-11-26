//! The authorization matrix: every declared route walked by anonymous callers
//! and by each role of docs/10-security.md §4, plus the default-deny guard for
//! a route registered without a declaration.

use fantatt_api::authz::{self, DeclaredRouter};
use fantatt_api::routes;
use fantatt_api::routes::{auth, health};
use fantatt_api::state::AppState;
use fantatt_core::authz::Capability;
use sqlx::PgPool;

mod common;

const LEAGUE_ID: &str = "lg_01ARZ3XK7M0C4V6B8N2QWE9RTA";
const LEAGUE_PATH: &str = "/leagues/lg_01ARZ3XK7M0C4V6B8N2QWE9RTA";
const LEAGUE_ROUTE: &str = "/leagues/{league_id}";
const SUPERADMIN_ID: &str = "usr_01ARZ3XK7M0C4V6B8N2QWE9RTA";

fn user_id(last: char) -> String {
    format!("usr_01ARZ3XK7M0C4V6B8N2QWE9RT{last}")
}

async fn league_roster() -> &'static str {
    "roster"
}

async fn invite_collaborator() -> &'static str {
    "invited"
}

async fn submit_squad() -> &'static str {
    "submitted"
}

async fn list_users() -> &'static str {
    "users"
}

async fn assign_roles() -> &'static str {
    "assigned"
}

async fn read_preferences() -> &'static str {
    "preferences"
}

async fn stub_handler() -> &'static str {
    "stub"
}

async fn ambiguity_queue() -> &'static str {
    "queue"
}

fn public_guard() -> authz::Guard {
    authz::guard(Capability::Public, authz::platform())
}

fn shell_routes(router: DeclaredRouter<AppState>) -> DeclaredRouter<AppState> {
    router
        .route("/healthz", authz::get(health::healthz, public_guard()))
        .route(
            "/auth/csrf",
            authz::get(auth::issue_csrf_token, public_guard()),
        )
        .route(
            LEAGUE_ROUTE,
            authz::get(
                league_roster,
                authz::guard(Capability::LeagueRead, authz::league("league_id")),
            ),
        )
}

fn league_mutation_routes(router: DeclaredRouter<AppState>) -> DeclaredRouter<AppState> {
    router
        .route(
            "/leagues/{league_id}/invitations",
            authz::post(
                invite_collaborator,
                authz::guard(Capability::RostersWrite, authz::league("league_id")),
            ),
        )
        .route(
            "/leagues/{league_id}/squads/me/submissions",
            authz::post(
                submit_squad,
                authz::guard(Capability::SquadOwn, authz::league("league_id")),
            ),
        )
}

fn identity_routes(router: DeclaredRouter<AppState>) -> DeclaredRouter<AppState> {
    router.route(
        "/admin/players/reconciliation",
        authz::get(
            ambiguity_queue,
            authz::guard(Capability::PlatformAdmin, authz::platform()),
        ),
    )
}

fn admin_routes(router: DeclaredRouter<AppState>) -> DeclaredRouter<AppState> {
    router
        .route(
            "/admin/users",
            authz::get(
                list_users,
                authz::guard(Capability::UsersRead, authz::platform()),
            ),
        )
        .route(
            "/admin/users/{user_id}/roles",
            authz::put(
                assign_roles,
                authz::guard(Capability::UsersWriteRoles, authz::platform()),
            ),
        )
}

fn account_routes(router: DeclaredRouter<AppState>) -> DeclaredRouter<AppState> {
    router.route(
        "/users/{user_id}/preferences",
        authz::get(
            read_preferences,
            authz::guard(Capability::Session, authz::account("user_id")),
        ),
    )
}

fn matrix_routes() -> (axum::Router<AppState>, authz::RouteTable) {
    let declared = shell_routes(DeclaredRouter::<AppState>::new());
    let declared = league_mutation_routes(declared);
    let declared = admin_routes(declared);
    let declared = identity_routes(declared);
    let declared = account_routes(declared);
    let (router, table) = declared.build();
    (
        router.route("/stub-undeclared", axum::routing::get(stub_handler)),
        table,
    )
}

fn build_matrix_router(state: AppState) -> axum::Router {
    let (router, table) = matrix_routes();
    routes::protected(router, &table, state)
}

async fn boot() -> common::Shell {
    let shell = common::spawn_with_env(
        &[("FANTATT__AUTHZ__SUPERADMINS", SUPERADMIN_ID)],
        build_matrix_router,
    )
    .await;
    seed_owner(&shell.pool).await;
    seed_league(&shell.pool).await;
    shell
}

async fn seed_owner(pool: &PgPool) {
    sqlx::query(
        "INSERT INTO fantatt.users (user_id, email, display_name)
         VALUES ($1, $2, $3)",
    )
    .bind(user_id('Z'))
    .bind("owner@example.org")
    .bind("league owner")
    .execute(pool)
    .await
    .expect("seed the owner");
}

async fn seed_league(pool: &PgPool) {
    sqlx::query(
        "INSERT INTO fantatt.leagues (league_id, name, slug, owner_id, season_year, budget_minor)
         VALUES ($1, $2, $3, $4, 2026, 100000)",
    )
    .bind(LEAGUE_ID)
    .bind("Matrix League")
    .bind("matrix-league")
    .bind(user_id('Z'))
    .execute(pool)
    .await
    .expect("seed the league");
}

/// Seeds a user holding a membership role and returns its session cookie.
async fn seed_caller(shell: &common::Shell, last: char, role: &str) -> String {
    seed_user(&shell.pool, last).await;
    seed_membership(&shell.pool, last, role).await;
    seed_principal(shell, &user_id(last)).await
}

async fn seed_user(pool: &PgPool, last: char) {
    sqlx::query(
        "INSERT INTO fantatt.users (user_id, email, display_name)
         VALUES ($1, $2, $3)",
    )
    .bind(user_id(last))
    .bind(format!("{last}@example.org"))
    .bind(format!("caller {last}"))
    .execute(pool)
    .await
    .expect("seed the user");
}

async fn seed_membership(pool: &PgPool, last: char, role: &str) {
    sqlx::query(
        "INSERT INTO fantatt.memberships (membership_id, league_id, user_id, role, state)
         VALUES ($1, $2, $3, $4, 'ACTIVE')",
    )
    .bind(format!("mbr_01ARZ3XK7M0C4V6B8N2QWE9RT{last}"))
    .bind(LEAGUE_ID)
    .bind(user_id(last))
    .bind(role)
    .execute(pool)
    .await
    .expect("seed the membership");
}

/// Mints a session and writes the principal id into its stored data.
async fn seed_principal(shell: &common::Shell, user_id: &str) -> String {
    let (cookie, _token) = common::issue_session(&shell.base_url).await;
    let session_id = common::cookie_value(&cookie).to_owned();
    let data = serde_json::to_vec(&serde_json::json!({ "principal_id": user_id }))
        .expect("encode the session data");
    sqlx::query("UPDATE tower_sessions.session SET data = $1 WHERE id = $2")
        .bind(data)
        .bind(&session_id)
        .execute(&shell.pool)
        .await
        .expect("bind the principal to the session");
    cookie
}

async fn send(request: reqwest::RequestBuilder) -> reqwest::Response {
    request.send().await.expect("the request is answered")
}

async fn read(shell: &common::Shell, path: &str, cookie: Option<&str>) -> reqwest::Response {
    let client = reqwest::Client::new();
    let request = client.get(format!("{}{}", shell.base_url, path));
    send(with_cookie(request, cookie)).await
}

async fn post(shell: &common::Shell, path: &str, cookie: Option<&str>) -> reqwest::Response {
    let client = reqwest::Client::new();
    let request = client.post(format!("{}{}", shell.base_url, path));
    send(with_cookie(request, cookie)).await
}

async fn put(shell: &common::Shell, path: &str, cookie: Option<&str>) -> reqwest::Response {
    let client = reqwest::Client::new();
    let request = client.put(format!("{}{}", shell.base_url, path));
    send(with_cookie(request, cookie)).await
}

fn with_cookie(
    mut request: reqwest::RequestBuilder,
    cookie: Option<&str>,
) -> reqwest::RequestBuilder {
    if let Some(cookie) = cookie {
        request = request.header(reqwest::header::COOKIE, cookie);
    }
    request
}

fn league_route(path: &str) -> String {
    format!("/leagues/{LEAGUE_ID}{path}")
}

#[tokio::test]
async fn a_league_read_route_serves_every_league_role() {
    let shell = boot().await;
    let anonymous = read(&shell, LEAGUE_PATH, None).await;
    assert_eq!(anonymous.status(), 403, "anonymous reads nothing scoped");
    for (last, role) in [
        ('B', "MEMBER"),
        ('C', "MANAGER"),
        ('D', "COLLABORATOR"),
        ('E', "LEAGUE_ADMIN"),
    ] {
        let cookie = seed_caller(&shell, last, role).await;
        let response = read(&shell, LEAGUE_PATH, Some(&cookie)).await;
        assert_eq!(response.status(), 200, "{role} reads its league");
    }
    let platform = seed_caller(&shell, 'A', "SUPERADMIN").await;
    let response = read(&shell, LEAGUE_PATH, Some(&platform)).await;
    assert_eq!(response.status(), 200, "the superadmin reads every league");
}

#[tokio::test]
async fn invitations_require_a_collaborator_or_above() {
    let shell = boot().await;
    let path = league_route("/invitations");
    let anonymous = post(&shell, &path, None).await;
    assert_eq!(anonymous.status(), 403, "anonymous invites nobody");
    for (last, role) in [('B', "MEMBER"), ('C', "MANAGER")] {
        let cookie = seed_caller(&shell, last, role).await;
        let response = post(&shell, &path, Some(&cookie)).await;
        assert_eq!(response.status(), 403, "{role} cannot invite");
    }
    for (last, role) in [('D', "COLLABORATOR"), ('E', "LEAGUE_ADMIN")] {
        let cookie = seed_caller(&shell, last, role).await;
        let response = post(&shell, &path, Some(&cookie)).await;
        assert_eq!(response.status(), 200, "{role} invites collaborators");
    }
    let platform = seed_caller(&shell, 'A', "SUPERADMIN").await;
    let response = post(&shell, &path, Some(&platform)).await;
    assert_eq!(response.status(), 200, "the superadmin invites");
}

#[tokio::test]
async fn submissions_require_a_manager_or_above() {
    let shell = boot().await;
    let path = league_route("/squads/me/submissions");
    for (last, role) in [('B', "MEMBER"), ('D', "COLLABORATOR")] {
        let cookie = seed_caller(&shell, last, role).await;
        let response = post(&shell, &path, Some(&cookie)).await;
        assert_eq!(response.status(), 403, "{role} cannot submit a squad");
    }
    for (last, role) in [('C', "MANAGER"), ('E', "LEAGUE_ADMIN")] {
        let cookie = seed_caller(&shell, last, role).await;
        let response = post(&shell, &path, Some(&cookie)).await;
        assert_eq!(response.status(), 200, "{role} submits the squad");
    }
}

#[tokio::test]
async fn admin_users_requires_the_superadmin_flag() {
    let shell = boot().await;
    let anonymous = read(&shell, "/admin/users", None).await;
    assert_eq!(
        anonymous.status(),
        403,
        "anonymous reaches no admin surface"
    );
    let admin = seed_caller(&shell, 'E', "LEAGUE_ADMIN").await;
    let response = read(&shell, "/admin/users", Some(&admin)).await;
    assert_eq!(
        response.status(),
        403,
        "a league admin is no platform superadmin"
    );
    let platform = seed_caller(&shell, 'A', "SUPERADMIN").await;
    let response = read(&shell, "/admin/users", Some(&platform)).await;
    assert_eq!(response.status(), 200, "the superadmin lists users");
}

#[tokio::test]
async fn role_assignment_requires_the_superadmin_flag() {
    let shell = boot().await;
    let path = format!("/admin/users/{}/roles", user_id('B'));
    let member = seed_caller(&shell, 'B', "MEMBER").await;
    let response = put(&shell, &path, Some(&member)).await;
    assert_eq!(response.status(), 403, "a member assigns no roles");
    let platform = seed_caller(&shell, 'A', "SUPERADMIN").await;
    let response = put(&shell, &path, Some(&platform)).await;
    assert_eq!(response.status(), 200, "the superadmin assigns roles");
}

#[tokio::test]
async fn the_ambiguity_queue_requires_the_superadmin_flag() {
    let shell = boot().await;
    let anonymous = read(&shell, "/admin/players/reconciliation", None).await;
    assert_eq!(anonymous.status(), 403, "anonymous opens no admin route");
    let admin = seed_caller(&shell, 'E', "LEAGUE_ADMIN").await;
    let response = read(&shell, "/admin/players/reconciliation", Some(&admin)).await;
    assert_eq!(
        response.status(),
        403,
        "a league admin is no platform superadmin"
    );
    let platform = seed_caller(&shell, 'A', "SUPERADMIN").await;
    let response = read(&shell, "/admin/players/reconciliation", Some(&platform)).await;
    assert_eq!(response.status(), 200, "the superadmin reads the queue");
}

#[tokio::test]
async fn a_session_route_scopes_to_the_own_account() {
    let shell = boot().await;
    let member = seed_caller(&shell, 'B', "MEMBER").await;
    let own = format!("/users/{}/preferences", user_id('B'));
    let response = read(&shell, &own, Some(&member)).await;
    assert_eq!(response.status(), 200, "a member reads own preferences");
    let foreign = format!("/users/{}/preferences", user_id('C'));
    let response = read(&shell, &foreign, Some(&member)).await;
    assert_eq!(
        response.status(),
        403,
        "another account's preferences stay closed"
    );
    let anonymous = read(&shell, &own, None).await;
    assert_eq!(anonymous.status(), 403, "anonymous reads no preferences");
    let platform = seed_caller(&shell, 'A', "SUPERADMIN").await;
    let response = read(&shell, &foreign, Some(&platform)).await;
    assert_eq!(response.status(), 200, "the superadmin reads every account");
}

#[tokio::test]
async fn an_undeclared_route_is_denied_for_every_caller() {
    let shell = boot().await;
    let anonymous = read(&shell, "/stub-undeclared", None).await;
    assert_eq!(anonymous.status(), 403, "anonymous hits the guard");
    let platform = seed_caller(&shell, 'A', "SUPERADMIN").await;
    let response = read(&shell, "/stub-undeclared", Some(&platform)).await;
    assert_eq!(
        response.status(),
        403,
        "even the superadmin cannot bypass the guard"
    );
    let body: serde_json::Value = response.json().await.expect("json");
    assert_eq!(body["code"], "ROUTE_UNDECLARED");
}

#[tokio::test]
async fn a_denial_names_the_missing_capability() {
    let shell = boot().await;
    let member = seed_caller(&shell, 'B', "MEMBER").await;
    let path = league_route("/invitations");
    let response = post(&shell, &path, Some(&member)).await;
    assert_eq!(response.status(), 403, "a member cannot invite");
    let body: serde_json::Value = response.json().await.expect("json");
    assert_eq!(
        body["type"],
        "https://fantatt.org/problems/capability-missing"
    );
    assert_eq!(body["code"], "CAPABILITY_MISSING");
    assert_eq!(body["details"]["capability"], "rosters:write");
}

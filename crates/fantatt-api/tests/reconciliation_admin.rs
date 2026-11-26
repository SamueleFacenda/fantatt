//! The identity reconciliation admin surface: the ambiguity queue and the
//! audited merge decision (docs/07-api-spec.md §12).

mod common;

use common::{Shell, issue_session, spawn_with_env};
use fantatt_api::routes;
use sqlx::PgPool;

const ACTOR_ID: &str = "usr_01ARZ3XK7M0C4V6B8N2QWE9RTA";
const LEAGUE_ADMIN_ID: &str = "usr_01ARZ3XK7M0C4V6B8N2QWE9RTB";
const LEAGUE_ID: &str = "lg_01ARZ3XK7M0C4V6B8N2QWE9RTA";
const ALIAS_ID: &str = "01ARZ3XK7M0C4V6B8N2QWE9RTA";
const QUEUE_ID: &str = "01ARZ3XK7M0C4V6B8N2QWE9RTQ";
const CANDIDATE: &str = "plr_01ARZ3XK7M0C4V6B8N2QWE9RTA";
const HOMONYM: &str = "plr_01ARZ3XK7M0C4V6B8N2QWE9RTB";
const FOREIGN: &str = "plr_01ARZ3XK7M0C4V6B8N2QWE9RTC";
const QUEUE_PATH: &str = "/api/v1/admin/players/reconciliation";

/// One authenticated caller: the shell it calls beside its session cookie and
/// CSRF token.
struct Caller<'a> {
    shell: &'a Shell,
    cookie: String,
    token: String,
}

impl Caller<'_> {
    async fn read_queue(&self) -> reqwest::Response {
        reqwest::Client::new()
            .get(format!("{}{QUEUE_PATH}", self.shell.base_url))
            .header(reqwest::header::COOKIE, &self.cookie)
            .send()
            .await
            .expect("the request is answered")
    }

    async fn decide(&self, queue_id: &str, player_id: &str) -> reqwest::Response {
        let path = format!("{}{QUEUE_PATH}/{queue_id}/resolve", self.shell.base_url);
        reqwest::Client::new()
            .post(path)
            .header(reqwest::header::COOKIE, &self.cookie)
            .header("x-csrf-token", &self.token)
            .json(&serde_json::json!({ "resolved_player_id": player_id }))
            .send()
            .await
            .expect("the request is answered")
    }
}

async fn boot() -> Shell {
    let shell = spawn_with_env(
        &[("FANTATT__AUTHZ__SUPERADMINS", ACTOR_ID)],
        routes::build_router,
    )
    .await;
    seed_admin(&shell.pool).await;
    seed_league_admin(&shell.pool).await;
    seed_ambiguity(&shell.pool).await;
    shell
}

async fn seed_admin(pool: &PgPool) {
    sqlx::query("INSERT INTO fantatt.users (user_id, email, display_name) VALUES ($1, $2, $3)")
        .bind(ACTOR_ID)
        .bind("admin@example.org")
        .bind("platform admin")
        .execute(pool)
        .await
        .expect("seed the admin");
}

async fn seed_league_admin(pool: &PgPool) {
    seed_user(pool, LEAGUE_ADMIN_ID, "league@example.org").await;
    seed_league(pool).await;
    seed_membership(pool).await;
}

async fn seed_league(pool: &PgPool) {
    sqlx::query(
        "INSERT INTO fantatt.leagues (league_id, name, slug, owner_id, season_year, budget_minor)
         VALUES ($1, $2, $3, $4, 2026, 100000)",
    )
    .bind(LEAGUE_ID)
    .bind("Admin League")
    .bind("admin-league")
    .bind(LEAGUE_ADMIN_ID)
    .execute(pool)
    .await
    .expect("seed the league");
}

async fn seed_membership(pool: &PgPool) {
    sqlx::query(
        "INSERT INTO fantatt.memberships (membership_id, league_id, user_id, role, state)
         VALUES ($1, $2, $3, 'LEAGUE_ADMIN', 'ACTIVE')",
    )
    .bind("mbr_01ARZ3XK7M0C4V6B8N2QWE9RTB")
    .bind(LEAGUE_ID)
    .bind(LEAGUE_ADMIN_ID)
    .execute(pool)
    .await
    .expect("seed the membership");
}

async fn seed_user(pool: &PgPool, user_id: &str, email: &str) {
    sqlx::query("INSERT INTO fantatt.users (user_id, email, display_name) VALUES ($1, $2, $3)")
        .bind(user_id)
        .bind(email)
        .bind(format!("caller {user_id}"))
        .execute(pool)
        .await
        .expect("seed the user");
}

async fn seed_ambiguity(pool: &PgPool) {
    for (player_id, name) in [(CANDIDATE, "Mario Rossi"), (HOMONYM, "Mario Rossi")] {
        seed_player(pool, player_id, name).await;
    }
    seed_player(pool, FOREIGN, "Luigi Bianchi").await;
    seed_alias_row(pool).await;
    seed_queue_row(pool).await;
}

async fn seed_player(pool: &PgPool, player_id: &str, full_name: &str) {
    sqlx::query(
        "INSERT INTO fantatt.players (player_id, full_name, gender) VALUES ($1, $2, 'MALE')",
    )
    .bind(player_id)
    .bind(full_name)
    .execute(pool)
    .await
    .expect("seed the player");
}

async fn seed_alias_row(pool: &PgPool) {
    sqlx::query(
        "INSERT INTO fantatt.identity_aliases
            (alias_id, player_id, raw_alias, normalized_alias, confidence, status)
         VALUES ($1, $2, 'ROSSI MARIO', 'Rossi Mario', 'AMBIGUOUS', 'AMBIGUOUS')",
    )
    .bind(ALIAS_ID)
    .bind(CANDIDATE)
    .execute(pool)
    .await
    .expect("seed the alias");
}

async fn seed_queue_row(pool: &PgPool) {
    sqlx::query(
        "INSERT INTO fantatt.identity_ambiguity_queue (queue_id, alias_id, candidate_player_ids)
         VALUES ($1, $2, ARRAY[$3::text, $4::text])",
    )
    .bind(QUEUE_ID)
    .bind(ALIAS_ID)
    .bind(CANDIDATE)
    .bind(HOMONYM)
    .execute(pool)
    .await
    .expect("seed the queue");
}

/// A session whose principal is the named user.
async fn caller<'a>(shell: &'a Shell, user_id: &str) -> Caller<'a> {
    let (cookie, token) = issue_session(&shell.base_url).await;
    let data = serde_json::to_vec(&serde_json::json!({ "principal_id": user_id })).expect("encode");
    sqlx::query("UPDATE tower_sessions.session SET data = $1 WHERE id = $2")
        .bind(data)
        .bind(common::cookie_value(&cookie))
        .execute(&shell.pool)
        .await
        .expect("bind the principal");
    Caller {
        shell,
        cookie,
        token,
    }
}

#[tokio::test]
async fn a_superadmin_reads_the_ambiguity_queue() {
    let shell = boot().await;
    let admin = caller(&shell, ACTOR_ID).await;
    let response = admin.read_queue().await;
    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.expect("json");
    let items = body["items"].as_array().expect("items");
    assert_eq!(items.len(), 1, "one ambiguity waits");
    assert_eq!(items[0]["queue_id"], QUEUE_ID);
    assert_eq!(items[0]["raw_alias"], "ROSSI MARIO");
    let candidates = items[0]["candidates"].as_array().expect("candidates");
    assert_eq!(candidates.len(), 2, "both homonyms are listed");
}

#[tokio::test]
async fn a_league_admin_opens_no_admin_route() {
    let shell = boot().await;
    let league_admin = caller(&shell, LEAGUE_ADMIN_ID).await;
    let read = league_admin.read_queue().await;
    assert_eq!(read.status(), 403);
    let body: serde_json::Value = read.json().await.expect("json");
    assert_eq!(
        body["type"],
        "https://fantatt.org/problems/capability-missing"
    );
    let settled = league_admin.decide(QUEUE_ID, CANDIDATE).await;
    assert_eq!(settled.status(), 403);
}

#[tokio::test]
async fn a_superadmin_settles_one_queued_ambiguity() {
    let shell = boot().await;
    let admin = caller(&shell, ACTOR_ID).await;
    let response = admin.decide(QUEUE_ID, CANDIDATE).await;
    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.expect("json");
    assert_eq!(body["queue_id"], QUEUE_ID);
    assert_eq!(body["resolved_player_id"], CANDIDATE);
    assert_resolved(&shell.pool).await;
}

async fn assert_resolved(pool: &PgPool) {
    assert_queue_settled(pool).await;
    assert_alias_moved(pool).await;
    assert_unprovisional(pool).await;
    assert_audited(pool).await;
}

async fn assert_queue_settled(pool: &PgPool) {
    let (status, resolved_by): (String, Option<String>) = sqlx::query_as(
        "SELECT status, resolved_by FROM fantatt.identity_ambiguity_queue WHERE queue_id = $1",
    )
    .bind(QUEUE_ID)
    .fetch_one(pool)
    .await
    .expect("fetch the queue row");
    assert_eq!(status, "RESOLVED", "the queue row settles");
    assert_eq!(resolved_by.as_deref(), Some(ACTOR_ID), "the actor is named");
}

async fn assert_alias_moved(pool: &PgPool) {
    let alias_player: String =
        sqlx::query_scalar("SELECT player_id FROM fantatt.identity_aliases WHERE alias_id = $1")
            .bind(ALIAS_ID)
            .fetch_one(pool)
            .await
            .expect("fetch the alias");
    assert_eq!(
        alias_player, CANDIDATE,
        "the alias moves to the chosen player"
    );
}

async fn assert_unprovisional(pool: &PgPool) {
    let provisional: bool =
        sqlx::query_scalar("SELECT provisional FROM fantatt.players WHERE player_id = $1")
            .bind(CANDIDATE)
            .fetch_one(pool)
            .await
            .expect("fetch the player");
    assert!(!provisional, "the chosen player stops being provisional");
}

async fn assert_audited(pool: &PgPool) {
    let audited: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM fantatt.audit_log
          WHERE actor_id = $1 AND action = 'identity.ambiguity.resolved' AND entity_id = $2",
    )
    .bind(ACTOR_ID)
    .bind(CANDIDATE)
    .fetch_one(pool)
    .await
    .expect("fetch the audit rows");
    assert_eq!(audited, 1, "the decision leaves one audit entry");
}

#[tokio::test]
async fn a_candidate_that_is_not_listed_is_refused() {
    let shell = boot().await;
    let admin = caller(&shell, ACTOR_ID).await;
    let response = admin.decide(QUEUE_ID, FOREIGN).await;
    assert_eq!(response.status(), 422);
    let body: serde_json::Value = response.json().await.expect("json");
    assert_eq!(body["code"], "E_CandidateNotListed");
    let pending: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM fantatt.identity_ambiguity_queue
          WHERE queue_id = $1 AND status = 'PENDING'",
    )
    .bind(QUEUE_ID)
    .fetch_one(&shell.pool)
    .await
    .expect("fetch the queue row");
    assert_eq!(pending, 1, "the ambiguity still waits for a decision");
}

#[tokio::test]
async fn an_ambiguity_is_settled_once() {
    let shell = boot().await;
    let admin = caller(&shell, ACTOR_ID).await;
    let settled = admin.decide(QUEUE_ID, HOMONYM).await;
    assert_eq!(settled.status(), 200);
    let repeated = admin.decide(QUEUE_ID, CANDIDATE).await;
    assert_eq!(repeated.status(), 422);
    let body: serde_json::Value = repeated.json().await.expect("json");
    assert_eq!(body["code"], "E_QueueNotPending");
}

#[tokio::test]
async fn an_unknown_queue_id_is_refused() {
    let shell = boot().await;
    let admin = caller(&shell, ACTOR_ID).await;
    let response = admin.decide("not-a-queue-id", CANDIDATE).await;
    assert_eq!(response.status(), 422);
    let body: serde_json::Value = response.json().await.expect("json");
    assert_eq!(body["code"], "E_QueueNotPending");
}

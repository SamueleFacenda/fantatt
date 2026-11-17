//! Schema-level conformance tests for the G0.5 migrations.

use fantatt_test_support::TestDatabase;

const ULID_ALPHABET: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

const EXPECTED_TABLES: [&str; 29] = [
    "fantatt.appearances",
    "fantatt.audit_log",
    "fantatt.bids",
    "fantatt.competitions",
    "fantatt.credentials",
    "fantatt.identity_aliases",
    "fantatt.identity_ambiguity_queue",
    "fantatt.ingest_jobs",
    "fantatt.ingest_ledger",
    "fantatt.invitations",
    "fantatt.leagues",
    "fantatt.lots",
    "fantatt.matchdays",
    "fantatt.memberships",
    "fantatt.notifications",
    "fantatt.offers",
    "fantatt.player_ratings",
    "fantatt.players",
    "fantatt.predictions",
    "fantatt.preferences",
    "fantatt.quarantine",
    "fantatt.slots",
    "fantatt.squads",
    "fantatt.submissions",
    "fantatt.teams",
    "fantatt.ties",
    "fantatt.tournaments",
    "fantatt.users",
    "tower_sessions.session",
];

/// A league to seed.
#[derive(Clone, Copy)]
struct LeaguePlan<'a> {
    owner: &'a str,
    mode: &'a str,
    budget: i64,
}

/// Builds a league seeded in one ownership mode.
fn league_plan<'a>(owner: &'a str, mode: &'a str, budget: i64) -> LeaguePlan<'a> {
    LeaguePlan {
        owner,
        mode,
        budget,
    }
}

/// The lot, league and manager a bid belongs to.
#[derive(Clone, Copy)]
struct Bidding<'a> {
    lot: &'a str,
    league: &'a str,
    manager: &'a str,
    state: &'a str,
}

impl<'a> Bidding<'a> {
    fn placed(lot: &'a str, league: &'a str, manager: &'a str) -> Self {
        Self {
            lot,
            league,
            manager,
            state: "PLACED",
        }
    }

    fn refunded(lot: &'a str, league: &'a str, manager: &'a str) -> Self {
        Self {
            lot,
            league,
            manager,
            state: "REFUNDED",
        }
    }
}

/// An audit entry to append.
struct AuditPlan<'a> {
    actor: &'a str,
    league: &'a str,
    action: &'a str,
}

fn audit_plan<'a>(actor: &'a str, league: &'a str, action: &'a str) -> AuditPlan<'a> {
    AuditPlan {
        actor,
        league,
        action,
    }
}

/// Seeding helper: hands out ULID identifiers and inserts the minimum rows a
/// constraint test needs.
struct Fixture<'a> {
    pool: &'a sqlx::PgPool,
    sequence: u64,
}

impl<'a> Fixture<'a> {
    fn new(pool: &'a sqlx::PgPool) -> Self {
        Self { pool, sequence: 0 }
    }

    fn identifier(&mut self, prefix: &str) -> String {
        self.sequence += 1;
        format!("{prefix}_{}", ulid_body(self.sequence))
    }

    fn bare_ulid(&mut self) -> String {
        self.sequence += 1;
        ulid_body(self.sequence)
    }

    async fn user(&mut self, email: &str) -> String {
        let id = self.identifier("usr");
        sqlx::query("INSERT INTO fantatt.users (user_id, email, display_name) VALUES ($1, $2, $3)")
            .bind(&id)
            .bind(email)
            .bind("Test Manager")
            .execute(self.pool)
            .await
            .expect("user insert");
        id
    }

    async fn league(&mut self, plan: LeaguePlan<'_>) -> String {
        let id = self.identifier("lg");
        sqlx::query(
            "INSERT INTO fantatt.leagues (league_id, name, slug, owner_id, season_year,
             ownership_mode, budget_minor) VALUES ($1, $2, $3, $4, 2026, $5, $6)",
        )
        .bind(&id)
        .bind(format!("League {}", self.sequence))
        .bind(format!("league-{}", self.sequence))
        .bind(plan.owner)
        .bind(plan.mode)
        .bind(plan.budget)
        .execute(self.pool)
        .await
        .expect("league insert");
        id
    }

    async fn squad(&mut self, league: &str, manager: &str) -> String {
        let id = self.identifier("sq");
        sqlx::query(
            "INSERT INTO fantatt.squads (squad_id, league_id, manager_id, month_year)
             VALUES ($1, $2, $3, '2026-03-01')",
        )
        .bind(&id)
        .bind(league)
        .bind(manager)
        .execute(self.pool)
        .await
        .expect("squad insert");
        id
    }

    async fn player(&mut self, name: &str) -> String {
        let id = self.identifier("plr");
        sqlx::query(
            "INSERT INTO fantatt.players (player_id, full_name, gender) VALUES ($1, $2, 'MALE')",
        )
        .bind(&id)
        .bind(name)
        .execute(self.pool)
        .await
        .expect("player insert");
        id
    }

    async fn lot(&mut self, league: &str, player: &str) -> String {
        let id = self.identifier("lot");
        sqlx::query("INSERT INTO fantatt.lots (lot_id, league_id, player_id) VALUES ($1, $2, $3)")
            .bind(&id)
            .bind(league)
            .bind(player)
            .execute(self.pool)
            .await
            .expect("lot insert");
        id
    }

    async fn slot(&mut self, squad: &str, player: &str) -> Result<u64, sqlx::Error> {
        let id = self.bare_ulid();
        sqlx::query(
            "INSERT INTO fantatt.slots (slot_id, squad_id, tier, position, is_bench, player_id)
             VALUES ($1, $2, 'C1', 0, false, $3)",
        )
        .bind(&id)
        .bind(squad)
        .bind(player)
        .execute(self.pool)
        .await
        .map(|result| result.rows_affected())
    }

    async fn bid(&mut self, context: Bidding<'_>, amount: i64) -> Result<u64, sqlx::Error> {
        let id = self.bare_ulid();
        sqlx::query(
            "INSERT INTO fantatt.bids (bid_id, lot_id, league_id, manager_id, amount_minor,
             state, idempotency_key) VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(&id)
        .bind(context.lot)
        .bind(context.league)
        .bind(context.manager)
        .bind(amount)
        .bind(context.state)
        .bind(format!("idem-{}", self.sequence))
        .execute(self.pool)
        .await
        .map(|result| result.rows_affected())
    }
}

fn ulid_body(sequence: u64) -> String {
    let mut remaining = sequence;
    let mut body: Vec<u8> = Vec::with_capacity(26);
    for _ in 0..26 {
        body.push(ULID_ALPHABET[(remaining % 32) as usize]);
        remaining /= 32;
    }
    String::from_utf8(body).expect("alphabet is ASCII")
}

async fn append_audit_entry(pool: &sqlx::PgPool, plan: AuditPlan<'_>) {
    sqlx::query(
        "INSERT INTO fantatt.audit_log (actor_id, league_id, action, prev_hash, entry_hash)
         VALUES ($1, $2, $3, 'placeholder', 'placeholder')",
    )
    .bind(plan.actor)
    .bind(plan.league)
    .bind(plan.action)
    .execute(pool)
    .await
    .expect("audit entry accepted");
}

async fn assert_version_count(pool: &sqlx::PgPool, expected: i64) {
    let count: (i64,) = sqlx::query_as("SELECT count(*) FROM _sqlx_migrations WHERE success")
        .fetch_one(pool)
        .await
        .expect("count applied migrations");
    assert_eq!(count.0, expected);
}

fn assert_check_violation(error: sqlx::Error) {
    let code = error
        .as_database_error()
        .and_then(|error| error.code())
        .map(|code| code.to_string());
    assert_eq!(
        code.as_deref(),
        Some("23514"),
        "expected 23514, got {error:?}"
    );
}

fn assert_append_only(error: sqlx::Error, operation: &str) {
    let message = error
        .as_database_error()
        .map(|error| error.message().to_owned())
        .unwrap_or_default();
    assert!(
        message.contains("append-only") && message.contains(operation),
        "expected append-only refusal for {operation}, got {message}"
    );
}

#[tokio::test]
async fn migrate_revert_migrate_round_trip() {
    let database = TestDatabase::new().await.expect("ephemeral database");
    fantatt_test_support::revert_all(database.pool())
        .await
        .expect("revert all migrations");
    assert_version_count(database.pool(), 0).await;
    fantatt_test_support::migrate(database.pool())
        .await
        .expect("re-apply all migrations");
    assert_version_count(database.pool(), 4).await;
}

#[tokio::test]
async fn expected_tables_exist() {
    let database = TestDatabase::new().await.expect("ephemeral database");
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT table_schema, table_name FROM information_schema.tables
         WHERE (table_schema = 'fantatt' AND table_type = 'BASE TABLE')
            OR (table_schema = 'tower_sessions' AND table_name = 'session')",
    )
    .fetch_all(database.pool())
    .await
    .expect("list tables");
    let mut found: Vec<String> = rows
        .into_iter()
        .map(|(schema, name)| format!("{schema}.{name}"))
        .collect();
    found.sort();
    let mut expected: Vec<String> = EXPECTED_TABLES
        .iter()
        .map(|name| (*name).to_owned())
        .collect();
    expected.sort();
    assert_eq!(found, expected);
}

#[tokio::test]
async fn exclusive_league_rejects_player_in_two_squads() {
    let database = TestDatabase::new().await.expect("ephemeral database");
    let mut fixture = Fixture::new(database.pool());
    let owner = fixture.user("owner@example.com").await;
    let rival = fixture.user("rival@example.com").await;
    let player = fixture.player("Bianchi Marco").await;
    let league = fixture.league(league_plan(&owner, "EXCLUSIVE", 1000)).await;
    let first = fixture.squad(&league, &owner).await;
    let second = fixture.squad(&league, &rival).await;
    fixture
        .slot(&first, &player)
        .await
        .expect("first slot accepted");
    let error = fixture
        .slot(&second, &player)
        .await
        .expect_err("second squad must reject the player");
    assert_check_violation(error);
}

#[tokio::test]
async fn shared_league_allows_player_in_two_squads() {
    let database = TestDatabase::new().await.expect("ephemeral database");
    let mut fixture = Fixture::new(database.pool());
    let owner = fixture.user("shared@example.com").await;
    let rival = fixture.user("shared-rival@example.com").await;
    let player = fixture.player("Verdi Luca").await;
    let league = fixture.league(league_plan(&owner, "SHARED", 1000)).await;
    let first = fixture.squad(&league, &owner).await;
    let second = fixture.squad(&league, &rival).await;
    fixture
        .slot(&first, &player)
        .await
        .expect("shared first slot");
    fixture
        .slot(&second, &player)
        .await
        .expect("shared league accepts the player in both squads");
}

#[tokio::test]
async fn budget_check_rejects_overdrawing_manager() {
    let database = TestDatabase::new().await.expect("ephemeral database");
    let mut fixture = Fixture::new(database.pool());
    let manager = fixture.user("bidder@example.com").await;
    let league = fixture
        .league(league_plan(&manager, "EXCLUSIVE", 500))
        .await;
    let player = fixture.player("Rossi Anna").await;
    let lot = fixture.lot(&league, &player).await;
    let context = Bidding::placed(&lot, &league, &manager);
    fixture.bid(context, 300).await.expect("first bid accepted");
    let error = fixture
        .bid(context, 300)
        .await
        .expect_err("second bid must exceed the budget");
    assert_check_violation(error);
}

#[tokio::test]
async fn refunded_bids_never_count_against_the_budget() {
    let database = TestDatabase::new().await.expect("ephemeral database");
    let mut fixture = Fixture::new(database.pool());
    let manager = fixture.user("refund@example.com").await;
    let league = fixture
        .league(league_plan(&manager, "EXCLUSIVE", 500))
        .await;
    let player = fixture.player("Neri Sara").await;
    let lot = fixture.lot(&league, &player).await;
    let refunded = Bidding::refunded(&lot, &league, &manager);
    fixture
        .bid(refunded, 900)
        .await
        .expect("refunded bid accepted");
    let placed = Bidding::placed(&lot, &league, &manager);
    fixture
        .bid(placed, 400)
        .await
        .expect("budget still available after a refund");
}

#[tokio::test]
async fn audit_log_is_append_only_and_chained() {
    let database = TestDatabase::new().await.expect("ephemeral database");
    let mut fixture = Fixture::new(database.pool());
    let actor = fixture.user("auditor@example.com").await;
    let league = fixture.league(league_plan(&actor, "SHARED", 1000)).await;
    append_audit_entry(
        database.pool(),
        audit_plan(&actor, &league, "league.created"),
    )
    .await;
    append_audit_entry(
        database.pool(),
        audit_plan(&actor, &league, "league.updated"),
    )
    .await;
    assert_audit_chain(database.pool()).await;
    assert_refuses_mutation(database.pool()).await;
}

async fn assert_audit_chain(pool: &sqlx::PgPool) {
    let chain: Vec<(String, String)> =
        sqlx::query_as("SELECT prev_hash, entry_hash FROM fantatt.audit_log ORDER BY entry_id")
            .fetch_all(pool)
            .await
            .expect("read audit chain");
    assert_eq!(chain[0].0, "GENESIS");
    assert_eq!(chain[1].0, chain[0].1);
    assert_ne!(chain[0].1, chain[1].1);
}

async fn assert_refuses_mutation(pool: &sqlx::PgPool) {
    let update = sqlx::query("UPDATE fantatt.audit_log SET action = 'tampered' WHERE entry_id = 1")
        .execute(pool)
        .await
        .expect_err("update must be refused");
    assert_append_only(update, "UPDATE");
    let delete = sqlx::query("DELETE FROM fantatt.audit_log WHERE entry_id = 1")
        .execute(pool)
        .await
        .expect_err("delete must be refused");
    assert_append_only(delete, "DELETE");
}

#[tokio::test]
async fn session_table_matches_store_schema() {
    let database = TestDatabase::new().await.expect("ephemeral database");
    let columns: Vec<(String, String)> = sqlx::query_as(
        "SELECT column_name, data_type FROM information_schema.columns
         WHERE table_schema = 'tower_sessions' AND table_name = 'session'
         ORDER BY ordinal_position",
    )
    .fetch_all(database.pool())
    .await
    .expect("list session columns");
    assert_eq!(
        columns,
        vec![
            ("id".to_owned(), "text".to_owned()),
            ("data".to_owned(), "bytea".to_owned()),
            (
                "expiry_date".to_owned(),
                "timestamp with time zone".to_owned()
            ),
        ]
    );
}

#[tokio::test]
async fn identifiers_must_be_prefixed_ulids() {
    let database = TestDatabase::new().await.expect("ephemeral database");
    let error = sqlx::query(
        "INSERT INTO fantatt.players (player_id, full_name, gender)
         VALUES ('not-an-ulid', 'Broken Id', 'MALE')",
    )
    .execute(database.pool())
    .await
    .expect_err("malformed identifiers must be rejected");
    assert_check_violation(error);
}
